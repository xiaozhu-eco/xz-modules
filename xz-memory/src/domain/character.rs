//! Character state management for novel writing.
//!
//! Characters are stored as [`Fact`]s via the layered [`MemorySystem`] using
//! the pattern `character:{character_id}` for reliable retrieval and FTS5
//! full-text search over names and aliases.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::MemoryError;
use crate::traits::MemorySystem;
use crate::types::fact::{Confidence, Fact, FactCategory, FactRecallOptions};
use crate::types::query::PageRequest;

/// A character snapshot at a point in the novel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterState {
    pub character_id: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub traits: HashMap<String, String>,
    pub relationships: HashMap<String, String>,
    pub last_appearance: Option<u32>,
    pub appearance_count: u32,
    pub arc_status: String,
    pub voice_profile: Option<String>,
    pub notes: String,
    pub updated_at: DateTime<Utc>,
}

/// Query options for finding characters.
#[derive(Debug, Clone)]
pub struct CharacterQuery {
    pub name_contains: Option<String>,
    pub status: Option<String>,
    pub recently_active: Option<u32>,
    pub limit: usize,
}

impl Default for CharacterQuery {
    fn default() -> Self {
        Self {
            name_contains: None,
            status: None,
            recently_active: None,
            limit: 100,
        }
    }
}

/// Manages character state for novel writing through the layered memory system.
///
/// Characters are stored as [`Fact`]s with the following mapping:
///
/// | Fact field   | Value                          |
/// |--------------|--------------------------------|
/// | `user_id`    | `novel_id`                     |
/// | `subject`    | `"character"`                  |
/// | `predicate`  | `"character:{character_id}"`   |
/// | `category`   | `Character`                    |
/// | `object`     | JSON serialized [`CharacterState`] |
///
/// This structure enables FTS5 full-text search over character names and
/// aliases while keeping predicate-based lookups for individual character
/// retrieval.
#[derive(Debug)]
pub struct CharacterMemory {
    memory: Arc<dyn MemorySystem>,
    novel_id: String,
}

impl CharacterMemory {
    /// Create a new `CharacterMemory` for the given novel.
    pub fn new(memory: Arc<dyn MemorySystem>, novel_id: &str) -> Self {
        Self {
            memory,
            novel_id: novel_id.to_string(),
        }
    }

    /// Insert or update a character state.
    ///
    /// Uses upsert semantics — if a character with the same `character_id`
    /// already exists, it is replaced.
    pub async fn upsert_character(&self, character: CharacterState) -> Result<(), MemoryError> {
        let fact = character_to_fact(&self.novel_id, &character)?;
        self.memory.remember_fact(fact).await?;
        Ok(())
    }

    /// Retrieve a single character by its ID.
    ///
    /// Returns `None` if no character with the given ID exists.
    pub async fn get_character(
        &self,
        character_id: &str,
    ) -> Result<Option<CharacterState>, MemoryError> {
        let options = FactRecallOptions {
            page: PageRequest {
                limit: 10,
                offset: 0,
            },
            ..Default::default()
        };

        let result = self
            .memory
            .recall_facts(&self.novel_id, character_id, &options)
            .await?;

        for fact in result.items {
            if fact.subject == "character"
                && fact.predicate == format!("character:{}", character_id)
            {
                return fact_to_character(fact);
            }
        }

        Ok(None)
    }

    /// Retrieve characters matching the given query filters.
    ///
    /// Fetches all characters for the novel and applies in-memory filtering
    /// by name, status, and recent activity. Results are sorted by
    /// `last_appearance` descending.
    pub async fn get_relevant_characters(
        &self,
        query: &CharacterQuery,
    ) -> Result<Vec<CharacterState>, MemoryError> {
        let search_query = query.name_contains.as_deref().unwrap_or("");
        let options = FactRecallOptions {
            page: PageRequest {
                limit: 1000,
                offset: 0,
            },
            categories: Some(vec![FactCategory::Character]),
            ..Default::default()
        };

        let result = self
            .memory
            .recall_facts(&self.novel_id, search_query, &options)
            .await?;

        let characters: Vec<CharacterState> = result
            .items
            .into_iter()
            .filter(|f| f.subject == "character")
            .filter_map(|f| fact_to_character(f).ok().flatten())
            .collect();

        let mut filtered = apply_character_query(characters, query);

        // Sort by last_appearance descending (None sorts last)
        filtered.sort_by_key(|c| std::cmp::Reverse(c.last_appearance));

        if query.limit > 0 {
            filtered.truncate(query.limit);
        }

        Ok(filtered)
    }

    /// List all characters for the novel.
    pub async fn get_all_characters(&self) -> Result<Vec<CharacterState>, MemoryError> {
        self.get_relevant_characters(&CharacterQuery::default())
            .await
    }

    /// Update character state after a chapter is written.
    ///
    /// For each character in `characters_in_chapter`: sets `last_appearance`
    /// to the chapter number, increments `appearance_count`, and transitions
    /// the arc status to `"active"` if it was `"introduced"` or `"dormant"`.
    ///
    /// For characters NOT in the chapter: transitions `"active"` characters
    /// to `"dormant"`.
    pub async fn update_after_chapter(
        &self,
        chapter_number: u32,
        characters_in_chapter: &[String],
    ) -> Result<(), MemoryError> {
        let all = self.get_all_characters().await?;

        for mut character in all {
            if characters_in_chapter.contains(&character.character_id) {
                character.last_appearance = Some(chapter_number);
                character.appearance_count =
                    character.appearance_count.saturating_add(1);
                if character.arc_status == "introduced" || character.arc_status == "dormant" {
                    character.arc_status = "active".to_string();
                }
            } else if character.arc_status == "active" {
                character.arc_status = "dormant".to_string();
            }
            character.updated_at = Utc::now();
            self.upsert_character(character).await?;
        }

        Ok(())
    }

    /// Delete a character by ID.
    pub async fn delete_character(&self, character_id: &str) -> Result<(), MemoryError> {
        let fact_id = format!("{}:{}", self.novel_id, character_id);
        self.memory.delete_fact(&fact_id).await
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Convert a [`CharacterState`] into a [`Fact`] for storage.
fn character_to_fact(novel_id: &str, character: &CharacterState) -> Result<Fact, MemoryError> {
    let object =
        serde_json::to_string(character).map_err(|e| MemoryError::serialization(e.to_string()))?;

    Ok(Fact {
        id: format!("{}:{}", novel_id, character.character_id),
        user_id: novel_id.to_string(),
        category: FactCategory::Character,
        subject: "character".to_string(),
        predicate: format!("character:{}", character.character_id),
        object,
        confidence: Confidence::High,
        source_session: None,
        created_at: character.updated_at.timestamp() as u64,
        updated_at: character.updated_at.timestamp() as u64,
        version: 1,
    })
}

/// Convert a [`Fact`] back into a [`CharacterState`].
///
/// Returns an error if the `object` field cannot be deserialized.
fn fact_to_character(fact: Fact) -> Result<Option<CharacterState>, MemoryError> {
    serde_json::from_str(&fact.object)
        .map(Some)
        .map_err(|e| MemoryError::serialization(e.to_string()))
}

/// Apply [`CharacterQuery`] filters to a vector of character states.
fn apply_character_query(
    chars: Vec<CharacterState>,
    query: &CharacterQuery,
) -> Vec<CharacterState> {
    chars
        .into_iter()
        .filter(|c| {
            if let Some(ref status) = query.status {
                if c.arc_status != *status {
                    return false;
                }
            }
            if let Some(ref recent) = query.recently_active {
                match c.last_appearance {
                    Some(ch) if ch >= *recent => {}
                    _ => return false,
                }
            }
            if let Some(ref name_part) = query.name_contains {
                let name_lower = c.name.to_lowercase();
                if !name_lower.contains(&name_part.to_lowercase()) {
                    let alias_match = c
                        .aliases
                        .iter()
                        .any(|a| a.to_lowercase().contains(&name_part.to_lowercase()));
                    if !alias_match {
                        return false;
                    }
                }
            }
            true
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::memory::InMemoryMemory;

    fn make_character(id: &str, name: &str) -> CharacterState {
        CharacterState {
            character_id: id.to_string(),
            name: name.to_string(),
            aliases: vec![],
            traits: HashMap::new(),
            relationships: HashMap::new(),
            last_appearance: None,
            appearance_count: 0,
            arc_status: "introduced".to_string(),
            voice_profile: None,
            notes: String::new(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_upsert_and_get_character() {
        let memory = Arc::new(InMemoryMemory::new());
        let cm = CharacterMemory::new(memory.clone(), "novel-1");

        let character = make_character("char-1", "Alice");
        cm.upsert_character(character.clone()).await.unwrap();

        let retrieved = cm.get_character("char-1").await.unwrap();
        assert!(retrieved.is_some());
        let c = retrieved.unwrap();
        assert_eq!(c.character_id, "char-1");
        assert_eq!(c.name, "Alice");
        assert_eq!(c.arc_status, "introduced");
    }

    #[tokio::test]
    async fn test_get_relevant_characters_by_name() {
        let memory = Arc::new(InMemoryMemory::new());
        let cm = CharacterMemory::new(memory.clone(), "novel-1");

        cm.upsert_character(make_character("char-1", "Alice"))
            .await
            .unwrap();
        cm.upsert_character(make_character("char-2", "Bob"))
            .await
            .unwrap();
        cm.upsert_character(make_character("char-3", "Charlie"))
            .await
            .unwrap();

        let query = CharacterQuery {
            name_contains: Some("Ali".to_string()),
            ..Default::default()
        };
        let results = cm.get_relevant_characters(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Alice");

        let query2 = CharacterQuery {
            name_contains: Some("Bo".to_string()),
            ..Default::default()
        };
        let results2 = cm.get_relevant_characters(&query2).await.unwrap();
        assert_eq!(results2.len(), 1);
        assert_eq!(results2[0].name, "Bob");
    }

    #[tokio::test]
    async fn test_character_not_found() {
        let memory = Arc::new(InMemoryMemory::new());
        let cm = CharacterMemory::new(memory.clone(), "novel-1");

        let result = cm.get_character("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_after_chapter() {
        let memory = Arc::new(InMemoryMemory::new());
        let cm = CharacterMemory::new(memory.clone(), "novel-1");

        cm.upsert_character(make_character("char-1", "Alice"))
            .await
            .unwrap();
        cm.upsert_character(make_character("char-2", "Bob"))
            .await
            .unwrap();

        cm.update_after_chapter(5, &["char-1".to_string()])
            .await
            .unwrap();

        let alice = cm.get_character("char-1").await.unwrap().unwrap();
        assert_eq!(alice.last_appearance, Some(5));
        assert_eq!(alice.appearance_count, 1);
        assert_eq!(alice.arc_status, "active");

        let bob = cm.get_character("char-2").await.unwrap().unwrap();
        assert_eq!(bob.last_appearance, None);
        assert_eq!(bob.appearance_count, 0);
        assert_eq!(bob.arc_status, "introduced");
    }

    #[tokio::test]
    async fn test_delete_character() {
        let memory = Arc::new(InMemoryMemory::new());
        let cm = CharacterMemory::new(memory.clone(), "novel-1");

        cm.upsert_character(make_character("char-1", "Alice"))
            .await
            .unwrap();
        cm.delete_character("char-1").await.unwrap();

        let result = cm.get_character("char-1").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_all_characters() {
        let memory = Arc::new(InMemoryMemory::new());
        let cm = CharacterMemory::new(memory.clone(), "novel-1");

        cm.upsert_character(make_character("char-1", "Alice"))
            .await
            .unwrap();
        cm.upsert_character(make_character("char-2", "Bob"))
            .await
            .unwrap();

        let all = cm.get_all_characters().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_search_by_alias() {
        let memory = Arc::new(InMemoryMemory::new());
        let cm = CharacterMemory::new(memory.clone(), "novel-1");

        let mut char_with_alias = make_character("char-1", "Alexander");
        char_with_alias.aliases = vec!["Alex".to_string(), "Al".to_string()];
        cm.upsert_character(char_with_alias).await.unwrap();

        let query = CharacterQuery {
            name_contains: Some("Alex".to_string()),
            ..Default::default()
        };
        let results = cm.get_relevant_characters(&query).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_filter_by_status() {
        let memory = Arc::new(InMemoryMemory::new());
        let cm = CharacterMemory::new(memory.clone(), "novel-1");

        let mut active_char = make_character("char-1", "Alice");
        active_char.arc_status = "active".to_string();
        cm.upsert_character(active_char).await.unwrap();

        cm.upsert_character(make_character("char-2", "Bob"))
            .await
            .unwrap();

        let query = CharacterQuery {
            status: Some("active".to_string()),
            ..Default::default()
        };
        let results = cm.get_relevant_characters(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Alice");
    }

    #[tokio::test]
    async fn test_update_after_chapter_resolves_dormant() {
        let memory = Arc::new(InMemoryMemory::new());
        let cm = CharacterMemory::new(memory.clone(), "novel-1");

        let mut dormant_char = make_character("char-1", "Alice");
        dormant_char.arc_status = "dormant".to_string();
        cm.upsert_character(dormant_char).await.unwrap();

        cm.update_after_chapter(3, &["char-1".to_string()])
            .await
            .unwrap();

        let alice = cm.get_character("char-1").await.unwrap().unwrap();
        assert_eq!(alice.arc_status, "active");
        assert_eq!(alice.last_appearance, Some(3));
    }

    #[tokio::test]
    async fn test_isolation_between_novels() {
        let memory = Arc::new(InMemoryMemory::new());
        let cm1 = CharacterMemory::new(memory.clone(), "novel-1");
        let cm2 = CharacterMemory::new(memory.clone(), "novel-2");

        cm1.upsert_character(make_character("char-1", "Alice"))
            .await
            .unwrap();
        cm2.upsert_character(make_character("char-1", "Bob"))
            .await
            .unwrap();

        let from_novel1 = cm1.get_character("char-1").await.unwrap().unwrap();
        assert_eq!(from_novel1.name, "Alice");

        let from_novel2 = cm2.get_character("char-1").await.unwrap().unwrap();
        assert_eq!(from_novel2.name, "Bob");
    }
}
