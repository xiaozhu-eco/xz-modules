//! In-memory store for testing (feature-gated behind `test-utils`).

use std::collections::HashMap;
use std::sync::RwLock;

use crate::error::MemoryError;
use crate::traits::MemorySystem;
use crate::types::fact::{
    CompactionResult, CompactionStrategy, Fact, FactCategory, FactPage, FactRecallOptions,
};
use crate::types::message::Message;
use crate::types::query::{
    ImportResult, MemoryExport, MemoryStats, MessagePage, PageRequest, UpsertResult,
};
use crate::types::session::{SessionSnapshot, SessionSummary};

/// In-memory memory system for unit testing.
#[derive(Debug, Default)]
pub struct InMemoryMemory {
    messages: RwLock<HashMap<String, Vec<Message>>>,
    summaries: RwLock<HashMap<String, SessionSummary>>,
    facts: RwLock<HashMap<String, Fact>>,
    #[cfg(feature = "vector-memory")]
    vectors: RwLock<HashMap<String, crate::types::vector::VectorEntry>>,
}

impl InMemoryMemory {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
#[allow(unused_variables)]
impl MemorySystem for InMemoryMemory {
    async fn append_message(&self, session_id: &str, msg: Message) -> Result<(), MemoryError> {
        let mut map = self.messages.write().unwrap();
        let messages = map.entry(session_id.to_string()).or_default();
        messages.push(msg);
        Ok(())
    }

    async fn get_recent_messages(
        &self,
        session_id: &str,
        n: usize,
    ) -> Result<Vec<Message>, MemoryError> {
        let map = self.messages.read().unwrap();
        let messages = map.get(session_id);
        match messages {
            Some(msgs) => {
                let len = msgs.len();
                let start = if len > n { len - n } else { 0 };
                Ok(msgs[start..].to_vec())
            }
            None => Ok(vec![]),
        }
    }

    async fn get_session_messages(
        &self,
        session_id: &str,
        page: PageRequest,
    ) -> Result<MessagePage, MemoryError> {
        let map = self.messages.read().unwrap();
        let messages = map.get(session_id);
        match messages {
            Some(msgs) => {
                let total = msgs.len();
                let start = page.offset;
                let end = (start + page.limit).min(total);
                let items = msgs[start..end].to_vec();
                let has_more = end < total;
                Ok(MessagePage { items, total, has_more })
            }
            None => Ok(MessagePage { items: vec![], total: 0, has_more: false }),
        }
    }

    async fn clear_short_term(&self, session_id: &str) -> Result<(), MemoryError> {
        let mut map = self.messages.write().unwrap();
        map.remove(session_id);
        Ok(())
    }

    async fn evict_oldest_messages(
        &self,
        session_id: &str,
        keep_count: usize,
    ) -> Result<usize, MemoryError> {
        let mut map = self.messages.write().unwrap();
        let messages = map.entry(session_id.to_string()).or_default();
        let total = messages.len();
        if total <= keep_count {
            return Ok(0);
        }
        let evicted = total - keep_count;
        // Keep only the most recent `keep_count` messages
        *messages = messages.split_off(evicted);
        Ok(evicted)
    }

    async fn update_summary(
        &self,
        _session_id: &str,
        summary: SessionSummary,
    ) -> Result<(), MemoryError> {
        let mut map = self.summaries.write().unwrap();
        map.insert(summary.session_id.clone(), summary);
        Ok(())
    }

    #[cfg(feature = "summary")]
    async fn get_or_create_summary(
        &self,
        session_id: &str,
        _provider: &dyn xz_provider::traits::LlmProvider,
    ) -> Result<SessionSummary, MemoryError> {
        let map = self.summaries.read().unwrap();
        match map.get(session_id) {
            Some(s) => Ok(s.clone()),
            None => Err(MemoryError::SessionNotFound(session_id.to_string())),
        }
    }

    async fn get_summary_history(
        &self,
        user_id: &str,
        limit: usize,
    ) -> Result<Vec<SessionSummary>, MemoryError> {
        let map = self.summaries.read().unwrap();
        let mut summaries: Vec<SessionSummary> =
            map.values().filter(|s| s.user_id == user_id).cloned().collect();
        summaries.sort_by_key(|s| s.updated_at);
        summaries.reverse();
        summaries.truncate(limit);
        Ok(summaries)
    }

    async fn remember_fact(&self, fact: Fact) -> Result<UpsertResult, MemoryError> {
        let mut map = self.facts.write().unwrap();
        // Check for existing fact with same (user_id, subject, predicate)
        let existing = map.values().find(|f| {
            f.user_id == fact.user_id && f.subject == fact.subject && f.predicate == fact.predicate
        });
        if let Some(old) = existing {
            if old.object == fact.object && old.confidence == fact.confidence {
                return Ok(UpsertResult::Unchanged);
            }
            let old_id = old.id.clone();
            map.remove(&old_id);
            map.insert(fact.id.clone(), fact);
            return Ok(UpsertResult::Updated {
                changed_fields: vec!["object".into(), "confidence".into()],
            });
        }
        map.insert(fact.id.clone(), fact);
        Ok(UpsertResult::Created)
    }

    async fn recall_facts(
        &self,
        user_id: &str,
        query: &str,
        options: &FactRecallOptions,
    ) -> Result<FactPage, MemoryError> {
        let map = self.facts.read().unwrap();
        let mut results: Vec<Fact> = map
            .values()
            .filter(|f| f.user_id == user_id)
            .filter(|f| {
                f.subject.contains(query) || f.predicate.contains(query) || f.object.contains(query)
            })
            .cloned()
            .collect();

        // Apply filters
        if let Some(ref min_conf) = options.min_confidence {
            results.retain(|f| f.confidence >= *min_conf);
        }
        if let Some(ref cats) = options.categories {
            results.retain(|f| cats.contains(&f.category));
        }

        let total = results.len();
        let start = options.page.offset;
        let end = (start + options.page.limit).min(total);
        let has_more = end < total;

        Ok(FactPage { items: results[start..end].to_vec(), total, has_more })
    }

    async fn get_user_preferences(&self, user_id: &str) -> Result<Vec<Fact>, MemoryError> {
        let map = self.facts.read().unwrap();
        Ok(map
            .values()
            .filter(|f| f.user_id == user_id && f.category == FactCategory::Preference)
            .cloned()
            .collect())
    }

    async fn delete_fact(&self, id: &str) -> Result<(), MemoryError> {
        let mut map = self.facts.write().unwrap();
        map.remove(id).map(|_| ()).ok_or_else(|| MemoryError::FactNotFound(id.to_string()))
    }

    async fn compact_facts(
        &self,
        user_id: &str,
        strategy: CompactionStrategy,
    ) -> Result<CompactionResult, MemoryError> {
        let mut map = self.facts.write().unwrap();
        let mut result = CompactionResult::default();

        match strategy {
            CompactionStrategy::MergeSimilar => {
                // Group by (user_id, subject, predicate), keep highest confidence
                let mut groups: HashMap<(String, String), Vec<String>> = HashMap::new();
                let ids: Vec<String> = map.keys().cloned().collect();
                for id in &ids {
                    if let Some(f) = map.get(id) {
                        if f.user_id == user_id {
                            let key = (f.subject.clone(), f.predicate.clone());
                            groups.entry(key).or_default().push(id.clone());
                        }
                    }
                }
                for (_key, group) in &groups {
                    if group.len() > 1 {
                        // Find the one with highest confidence
                        let best_id = group
                            .iter()
                            .filter_map(|id| map.get(id))
                            .max_by_key(|f| f.confidence)
                            .map(|f| f.id.clone());
                        // Remove the rest
                        for id in group {
                            if Some(id.clone()) != best_id {
                                map.remove(id);
                                result.facts_merged += 1;
                            }
                        }
                    }
                }
            }
            CompactionStrategy::RemoveLowConfidence(threshold) => {
                let ids: Vec<String> = map.keys().cloned().collect();
                for id in ids {
                    if let Some(f) = map.get(&id) {
                        if f.user_id == user_id && f.confidence.as_f32() < threshold {
                            map.remove(&id);
                            result.facts_removed += 1;
                        }
                    }
                }
            }
            CompactionStrategy::RemoveOld(before_ts) => {
                let ids: Vec<String> = map.keys().cloned().collect();
                for id in ids {
                    if let Some(f) = map.get(&id) {
                        if f.user_id == user_id
                            && f.updated_at < before_ts
                            && f.confidence.as_f32() < 0.6
                        {
                            map.remove(&id);
                            result.facts_removed += 1;
                        }
                    }
                }
            }
        }

        result.facts_kept = map.values().filter(|f| f.user_id == user_id).count();
        Ok(result)
    }

    #[cfg(feature = "vector-memory")]
    async fn store_vector(
        &self,
        entry: crate::types::vector::VectorEntry,
    ) -> Result<(), MemoryError> {
        let mut map = self.vectors.write().unwrap();
        map.insert(entry.id.clone(), entry);
        Ok(())
    }

    #[cfg(feature = "vector-memory")]
    async fn search_vector(
        &self,
        query: &[f32],
        limit: usize,
        threshold: f32,
    ) -> Result<Vec<crate::types::vector::SearchResult>, MemoryError> {
        let map = self.vectors.read().unwrap();
        let mut results: Vec<crate::types::vector::SearchResult> = Vec::new();

        for entry in map.values() {
            if entry.vector.len() != query.len() {
                continue;
            }
            let similarity = cosine_similarity(query, &entry.vector);
            if similarity >= threshold {
                results.push(crate::types::vector::SearchResult {
                    id: entry.id.clone(),
                    score: similarity,
                    metadata: entry.metadata.clone(),
                    content: entry.content.clone(),
                    channel: None,
                });
            }
        }

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);

        Ok(results)
    }

    #[cfg(feature = "vector-memory")]
    async fn delete_vector(&self, id: &str) -> Result<(), MemoryError> {
        let mut map = self.vectors.write().unwrap();
        map.remove(id);
        Ok(())
    }

    async fn stats(&self, user_id: &str) -> Result<MemoryStats, MemoryError> {
        let total_vectors = {
            #[cfg(feature = "vector-memory")]
            {
                self.vectors
                    .read()
                    .unwrap()
                    .values()
                    .filter(|v| v.metadata.get("user_id").map(|s| s.as_str()) == Some(user_id))
                    .count()
            }
            #[cfg(not(feature = "vector-memory"))]
            {
                0_usize
            }
        };

        Ok(MemoryStats {
            total_sessions: self.messages.read().unwrap().len(),
            total_messages: self.messages.read().unwrap().values().map(|v| v.len()).sum(),
            total_facts: self.facts.read().unwrap().len(),
            total_vectors,
            total_tokens_approx: 0,
            db_size_bytes: 0,
        })
    }

    async fn export(&self, user_id: &str) -> Result<MemoryExport, MemoryError> {
        let messages_map = self.messages.read().unwrap();
        let sessions: Vec<SessionSnapshot> = messages_map
            .iter()
            .filter(|(_, msgs)| msgs.first().map(|m| m.user_id == user_id).unwrap_or(false))
            .map(|(sid, msgs)| SessionSnapshot {
                session_id: sid.clone(),
                messages: msgs.clone(),
                summary: self.summaries.read().unwrap().get(sid).cloned(),
            })
            .collect();

        let facts: Vec<Fact> =
            self.facts.read().unwrap().values().filter(|f| f.user_id == user_id).cloned().collect();

        let vectors: Vec<crate::types::vector::VectorEntry> = {
            #[cfg(feature = "vector-memory")]
            {
                self.vectors
                    .read()
                    .unwrap()
                    .values()
                    .filter(|v| v.metadata.get("user_id").map(|s| s.as_str()) == Some(user_id))
                    .cloned()
                    .collect()
            }
            #[cfg(not(feature = "vector-memory"))]
            {
                vec![]
            }
        };

        Ok(MemoryExport {
            version: "1.0".into(),
            user_id: user_id.to_string(),
            exported_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            sessions,
            facts,
            vectors,
        })
    }

    async fn import(&self, data: MemoryExport) -> Result<ImportResult, MemoryError> {
        let sessions_count = data.sessions.len();
        let facts_count = data.facts.len();
        let vectors_count = data.vectors.len();

        for snapshot in &data.sessions {
            for msg in &snapshot.messages {
                self.append_message(&snapshot.session_id, msg.clone()).await?;
            }
            if let Some(ref summary) = snapshot.summary {
                self.update_summary(&snapshot.session_id, summary.clone()).await?;
            }
        }

        for fact in &data.facts {
            self.remember_fact(fact.clone()).await?;
        }

        #[cfg(feature = "vector-memory")]
        for vector in &data.vectors {
            self.store_vector(vector.clone()).await?;
        }

        Ok(ImportResult {
            sessions_imported: sessions_count,
            facts_imported: facts_count,
            vectors_imported: vectors_count,
        })
    }
}

#[cfg(feature = "vector-memory")]
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }
    dot / (mag_a * mag_b)
}
