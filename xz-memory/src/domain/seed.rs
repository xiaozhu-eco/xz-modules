//! Narrative seed management.
//!
//! Seeds are plot elements, character traits, or world rules that must be
//! incorporated into generated novel chapters. They are persisted as
//! [`Fact`]s via the layered [`MemorySystem`] using the predicate pattern
//! `seed:{seed_id}` for reliable retrieval.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::MemoryError;
use crate::traits::MemorySystem;
use crate::types::fact::{Confidence, Fact, FactCategory, FactRecallOptions};
use crate::types::query::PageRequest;

/// A narrative seed — a plot element, character trait, or world rule.
///
/// Seeds encode narrative requirements that must be fulfilled by generated
/// chapters. Each seed belongs to a single novel and carries metadata such
/// as urgency, a chapter deadline, and completion tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeSeed {
    /// Unique identifier for this seed.
    pub seed_id: String,
    /// The novel this seed belongs to.
    pub novel_id: String,
    /// Semantic category of the seed.
    pub seed_type: SeedType,
    /// The actual seed content or instruction text.
    pub content: String,
    /// Origin of the seed: `"user"`, `"system"`, or `"derived"`.
    pub source: String,
    /// How urgently the seed needs to be addressed.
    pub urgency: SeedUrgency,
    /// Chapter number by which this seed must be addressed (if any).
    pub chapter_deadline: Option<u32>,
    /// Whether the seed must appear in generated output.
    pub is_mandatory: bool,
    /// Whether the seed has been fulfilled.
    pub is_completed: bool,
    /// Chapter number at which this seed was completed.
    pub completed_at: Option<u32>,
    /// IDs of related seeds.
    pub related_seeds: Vec<String>,
    /// When the seed was created.
    pub created_at: DateTime<Utc>,
    /// When the seed was last updated.
    pub updated_at: DateTime<Utc>,
}

/// Semantic category of a narrative seed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SeedType {
    /// A key plot event or beat.
    PlotPoint,
    /// A character personality trait or backstory element.
    CharacterTrait,
    /// A rule governing the story world.
    WorldRule,
    /// A hint or setup for future events.
    Foreshadowing,
    /// A recurring theme or motif.
    ThematicElement,
    /// A stylistic constraint or directive.
    StyleDirective,
}

/// Urgency level of a narrative seed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SeedUrgency {
    /// Optional — may be addressed if convenient.
    Low,
    /// Should be addressed soon.
    Medium,
    /// Important — should be addressed in the current or next chapter.
    High,
    /// Must be addressed immediately.
    Critical,
}

/// Filter criteria for querying narrative seeds.
///
/// All fields are optional; only provided filters are applied. A default
/// [`SeedQuery`] returns all seeds with default limit of 100.
#[derive(Debug, Clone)]
pub struct SeedQuery {
    /// Only return seeds of this type.
    pub seed_type: Option<SeedType>,
    /// Only return seeds with this urgency.
    pub urgency: Option<SeedUrgency>,
    /// Only return seeds that are mandatory.
    pub mandatory_only: bool,
    /// Only return seeds with High or Critical urgency.
    pub urgent_only: bool,
    /// Whether to include completed seeds (default: exclude).
    pub include_completed: bool,
    /// Only return seeds with a deadline before this chapter number.
    pub chapter_deadline_before: Option<u32>,
    /// Maximum number of seeds to return (0 = no limit).
    pub limit: usize,
}

impl Default for SeedQuery {
    fn default() -> Self {
        Self {
            seed_type: None,
            urgency: None,
            mandatory_only: false,
            urgent_only: false,
            include_completed: false,
            chapter_deadline_before: None,
            limit: 100,
        }
    }
}

/// Manages narrative seeds through the layered memory system.
///
/// Seeds are stored as [`Fact`]s with the following mapping:
///
/// | Fact field   | Value                        |
/// |--------------|------------------------------|
/// | `user_id`    | `novel_id`                   |
/// | `subject`    | `"narrative_seed"`          |
/// | `predicate`  | `"seed:{seed_id}"`          |
/// | `category`   | `Custom("Seed")`            |
/// | `object`     | JSON serialized [`NarrativeSeed`] |
///
/// This structure enables FTS5 full-text search over seed content while keeping
/// predicate-based lookups for individual seed retrieval.
#[derive(Debug)]
pub struct SeedMemory {
    memory: Arc<dyn MemorySystem>,
    novel_id: String,
}

impl SeedMemory {
    /// Create a new `SeedMemory` for the given novel.
    pub fn new(memory: Arc<dyn MemorySystem>, novel_id: &str) -> Self {
        Self { memory, novel_id: novel_id.to_string() }
    }

    /// Insert or update a narrative seed.
    ///
    /// Uses upsert semantics — if a seed with the same `seed_id` already exists,
    /// it is replaced.
    pub async fn upsert_seed(&self, seed: NarrativeSeed) -> Result<(), MemoryError> {
        let fact = seed_to_fact(&self.novel_id, &seed)?;
        self.memory.remember_fact(fact).await?;
        Ok(())
    }

    /// Retrieve a single seed by its ID.
    ///
    /// Returns `None` if no seed with the given ID exists.
    pub async fn get_seed(&self, seed_id: &str) -> Result<Option<NarrativeSeed>, MemoryError> {
        let options =
            FactRecallOptions { page: PageRequest { limit: 10, offset: 0 }, ..Default::default() };

        // Search using seed_id as the query — it will match in predicate "seed:{seed_id}"
        let result = self.memory.recall_facts(&self.novel_id, seed_id, &options).await?;

        for fact in result.items {
            if fact.subject == "narrative_seed" && fact.predicate == format!("seed:{}", seed_id) {
                return fact_to_seed(fact);
            }
        }

        Ok(None)
    }

    /// Retrieve seeds matching the given query filters.
    ///
    /// Fetches all seeds for the novel and applies in-memory filtering.
    /// This is acceptable because the number of seeds per novel is expected
    /// to be well within reasonable limits.
    pub async fn get_seeds(&self, query: &SeedQuery) -> Result<Vec<NarrativeSeed>, MemoryError> {
        let options = FactRecallOptions {
            page: PageRequest { limit: 1000, offset: 0 },
            ..Default::default()
        };

        let result = self.memory.recall_facts(&self.novel_id, "", &options).await?;

        let seeds: Vec<NarrativeSeed> = result
            .items
            .into_iter()
            .filter(|f| f.subject == "narrative_seed")
            .filter_map(|f| fact_to_seed(f).ok().flatten())
            .collect();

        let mut filtered = apply_seed_query(seeds, query);

        if query.limit > 0 {
            filtered.truncate(query.limit);
        }

        Ok(filtered)
    }

    /// Get mandatory seeds that are urgent and not yet completed.
    ///
    /// Convenience method equivalent to a [`SeedQuery`] with:
    /// - `mandatory_only = true`
    /// - `urgent_only = true`
    /// - `include_completed = false`
    pub async fn get_mandatory_seeds(&self) -> Result<Vec<NarrativeSeed>, MemoryError> {
        let query = SeedQuery {
            mandatory_only: true,
            urgent_only: true,
            include_completed: false,
            ..Default::default()
        };
        self.get_seeds(&query).await
    }

    /// Mark a seed as completed at the given chapter number.
    ///
    /// Returns [`MemoryError::FactNotFound`] if the seed does not exist.
    pub async fn mark_completed(
        &self,
        seed_id: &str,
        chapter_number: u32,
    ) -> Result<(), MemoryError> {
        let mut seed = self
            .get_seed(seed_id)
            .await?
            .ok_or_else(|| MemoryError::FactNotFound(seed_id.to_string()))?;

        seed.is_completed = true;
        seed.completed_at = Some(chapter_number);
        seed.updated_at = Utc::now();

        self.upsert_seed(seed).await
    }

    /// Update seeds after a chapter is generated.
    ///
    /// Any seed whose `chapter_deadline` is before the given `chapter_number`
    /// and is not yet completed will have its urgency escalated (e.g., Low →
    /// Medium, Medium → High, High → Critical). Critical seeds remain Critical.
    pub async fn update_after_chapter(&self, chapter_number: u32) -> Result<(), MemoryError> {
        let query = SeedQuery { include_completed: true, ..Default::default() };
        let seeds = self.get_seeds(&query).await?;

        for mut seed in seeds {
            if seed.is_completed {
                continue;
            }
            if let Some(deadline) = seed.chapter_deadline {
                if deadline < chapter_number {
                    let escalated = escalate_urgency(&seed.urgency);
                    if escalated != seed.urgency {
                        seed.urgency = escalated;
                        seed.updated_at = Utc::now();
                        self.upsert_seed(seed).await?;
                    }
                }
            }
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Convert a [`NarrativeSeed`] into a [`Fact`] for storage.
fn seed_to_fact(novel_id: &str, seed: &NarrativeSeed) -> Result<Fact, MemoryError> {
    let object =
        serde_json::to_string(seed).map_err(|e| MemoryError::serialization(e.to_string()))?;

    Ok(Fact {
        id: seed.seed_id.clone(),
        user_id: novel_id.to_string(),
        category: FactCategory::Custom("Seed".to_string()),
        subject: "narrative_seed".to_string(),
        predicate: format!("seed:{}", seed.seed_id),
        object,
        confidence: Confidence::High,
        source_session: None,
        created_at: seed.created_at.timestamp() as u64,
        updated_at: seed.updated_at.timestamp() as u64,
        version: 1,
    })
}

/// Convert a [`Fact`] back into a [`NarrativeSeed`].
///
/// Returns an error if the `object` field cannot be deserialized.
fn fact_to_seed(fact: Fact) -> Result<Option<NarrativeSeed>, MemoryError> {
    serde_json::from_str(&fact.object)
        .map(Some)
        .map_err(|e| MemoryError::serialization(e.to_string()))
}

/// Apply [`SeedQuery`] filters to a vector of seeds.
fn apply_seed_query(seeds: Vec<NarrativeSeed>, query: &SeedQuery) -> Vec<NarrativeSeed> {
    seeds
        .into_iter()
        .filter(|s| {
            if !query.include_completed && s.is_completed {
                return false;
            }
            if query.mandatory_only && !s.is_mandatory {
                return false;
            }
            if query.urgent_only && s.urgency < SeedUrgency::High {
                return false;
            }
            if let Some(ref st) = query.seed_type {
                if s.seed_type != *st {
                    return false;
                }
            }
            if let Some(ref u) = query.urgency {
                if s.urgency != *u {
                    return false;
                }
            }
            if let Some(deadline) = query.chapter_deadline_before {
                match s.chapter_deadline {
                    Some(d) if d < deadline => {} // keep
                    _ => return false,
                }
            }
            true
        })
        .collect()
}

/// Increase urgency by one level.
fn escalate_urgency(urgency: &SeedUrgency) -> SeedUrgency {
    match urgency {
        SeedUrgency::Low => SeedUrgency::Medium,
        SeedUrgency::Medium => SeedUrgency::High,
        SeedUrgency::High => SeedUrgency::Critical,
        SeedUrgency::Critical => SeedUrgency::Critical,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::memory::InMemoryMemory;

    fn create_seed(
        seed_id: &str,
        novel_id: &str,
        mandatory: bool,
        urgency: SeedUrgency,
    ) -> NarrativeSeed {
        NarrativeSeed {
            seed_id: seed_id.to_string(),
            novel_id: novel_id.to_string(),
            seed_type: SeedType::PlotPoint,
            content: format!("Content for {}", seed_id),
            source: "user".to_string(),
            urgency,
            chapter_deadline: None,
            is_mandatory: mandatory,
            is_completed: false,
            completed_at: None,
            related_seeds: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn create_seed_full(
        seed_id: &str,
        novel_id: &str,
        seed_type: SeedType,
        mandatory: bool,
        urgency: SeedUrgency,
        deadline: Option<u32>,
    ) -> NarrativeSeed {
        NarrativeSeed {
            seed_id: seed_id.to_string(),
            novel_id: novel_id.to_string(),
            seed_type,
            content: format!("Content for {}", seed_id),
            source: "user".to_string(),
            urgency,
            chapter_deadline: deadline,
            is_mandatory: mandatory,
            is_completed: false,
            completed_at: None,
            related_seeds: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_upsert_and_get_seed() -> Result<(), MemoryError> {
        let mem = Arc::new(InMemoryMemory::new());
        let sm = SeedMemory::new(mem, "novel-1");

        let seed = create_seed("s1", "novel-1", false, SeedUrgency::Low);
        sm.upsert_seed(seed.clone()).await?;

        let retrieved = sm.get_seed("s1").await?;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().seed_id, "s1");

        Ok(())
    }

    #[tokio::test]
    async fn test_get_mandatory_seeds() -> Result<(), MemoryError> {
        let mem = Arc::new(InMemoryMemory::new());
        let sm = SeedMemory::new(mem, "novel-2");

        // Mandatory urgent seed
        let s1 = create_seed("s1", "novel-2", true, SeedUrgency::High);
        sm.upsert_seed(s1).await?;

        // Mandatory low urgency (should NOT appear)
        let s2 = create_seed("s2", "novel-2", true, SeedUrgency::Low);
        sm.upsert_seed(s2).await?;

        // Non-mandatory urgent (should NOT appear)
        let s3 = create_seed("s3", "novel-2", false, SeedUrgency::Critical);
        sm.upsert_seed(s3).await?;

        let mandatory = sm.get_mandatory_seeds().await?;
        assert_eq!(mandatory.len(), 1);
        assert_eq!(mandatory[0].seed_id, "s1");

        Ok(())
    }

    #[tokio::test]
    async fn test_seed_query_filter() -> Result<(), MemoryError> {
        let mem = Arc::new(InMemoryMemory::new());
        let sm = SeedMemory::new(mem, "novel-3");

        let s_plot =
            create_seed_full("sp", "novel-3", SeedType::PlotPoint, false, SeedUrgency::Low, None);
        let s_char = create_seed_full(
            "sc",
            "novel-3",
            SeedType::CharacterTrait,
            false,
            SeedUrgency::Medium,
            None,
        );
        let s_world = create_seed_full(
            "sw",
            "novel-3",
            SeedType::WorldRule,
            true,
            SeedUrgency::Critical,
            None,
        );

        sm.upsert_seed(s_plot).await?;
        sm.upsert_seed(s_char).await?;
        sm.upsert_seed(s_world).await?;

        // Filter by type
        let q = SeedQuery {
            seed_type: Some(SeedType::CharacterTrait),
            include_completed: true,
            ..Default::default()
        };
        let results = sm.get_seeds(&q).await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].seed_id, "sc");

        // Filter by urgency
        let q = SeedQuery {
            urgency: Some(SeedUrgency::Critical),
            include_completed: true,
            ..Default::default()
        };
        let results = sm.get_seeds(&q).await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].seed_id, "sw");

        // Filter by mandatory_only
        let q = SeedQuery { mandatory_only: true, include_completed: true, ..Default::default() };
        let results = sm.get_seeds(&q).await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].seed_id, "sw");

        Ok(())
    }

    #[tokio::test]
    async fn test_mark_completed() -> Result<(), MemoryError> {
        let mem = Arc::new(InMemoryMemory::new());
        let sm = SeedMemory::new(mem, "novel-4");

        let seed = create_seed("sc1", "novel-4", true, SeedUrgency::High);
        sm.upsert_seed(seed).await?;

        sm.mark_completed("sc1", 5).await?;

        let retrieved = sm.get_seed("sc1").await?.unwrap();
        assert!(retrieved.is_completed);
        assert_eq!(retrieved.completed_at, Some(5));

        Ok(())
    }

    #[tokio::test]
    async fn test_seed_not_found() -> Result<(), MemoryError> {
        let mem = Arc::new(InMemoryMemory::new());
        let sm = SeedMemory::new(mem, "novel-5");

        let retrieved = sm.get_seed("nonexistent").await?;
        assert!(retrieved.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_update_after_chapter_escalates() -> Result<(), MemoryError> {
        let mem = Arc::new(InMemoryMemory::new());
        let sm = SeedMemory::new(mem, "novel-6");

        let s_low = create_seed_full(
            "slow",
            "novel-6",
            SeedType::PlotPoint,
            false,
            SeedUrgency::Low,
            Some(3),
        );
        let s_high = create_seed_full(
            "shigh",
            "novel-6",
            SeedType::PlotPoint,
            false,
            SeedUrgency::High,
            Some(3),
        );
        let s_no_deadline =
            create_seed_full("snd", "novel-6", SeedType::PlotPoint, false, SeedUrgency::Low, None);

        sm.upsert_seed(s_low).await?;
        sm.upsert_seed(s_high).await?;
        sm.upsert_seed(s_no_deadline).await?;

        // Chapter 5 is past deadline 3
        sm.update_after_chapter(5).await?;

        let all =
            sm.get_seeds(&SeedQuery { include_completed: true, ..Default::default() }).await?;

        for seed in &all {
            match seed.seed_id.as_str() {
                "slow" => {
                    assert_eq!(seed.urgency, SeedUrgency::Medium, "Low should escalate to Medium")
                }
                "shigh" => assert_eq!(
                    seed.urgency,
                    SeedUrgency::Critical,
                    "High should escalate to Critical"
                ),
                "snd" => {
                    assert_eq!(seed.urgency, SeedUrgency::Low, "No deadline should remain Low")
                }
                other => panic!("unexpected seed: {}", other),
            }
        }

        Ok(())
    }
}
