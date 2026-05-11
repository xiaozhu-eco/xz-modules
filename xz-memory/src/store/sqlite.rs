#[cfg(feature = "vector-memory")]
use std::sync::Arc;

use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use tracing::{debug, info, warn};

use crate::config::MemoryConfig;
use crate::error::MemoryError;
use crate::traits::MemorySystem;
use crate::types::fact::{
    CompactionResult, CompactionStrategy, Fact, FactCategory, FactPage, FactRecallOptions,
    Confidence,
};
use crate::types::message::{Message, Role};
use crate::types::query::{ImportResult, MemoryExport, MemoryStats, MessagePage, PageRequest, UpsertResult};
use crate::types::session::{SessionSummary, SessionSnapshot};

use super::sqlite_schema::{DDL, FTS_TRIGGERS};

/// SQLite-backed memory system implementation.
#[derive(Debug)]
pub struct SqliteMemory {
    pool: SqlitePool,
    #[cfg(feature = "vector-memory")]
    embed: Option<Arc<dyn xz_embed::traits::EmbeddingModel>>,
    config: MemoryConfig,
}

impl SqliteMemory {
    /// Create a new SqliteMemory with the given database path and configuration.
    pub async fn new(path: &str, config: MemoryConfig) -> Result<Self, MemoryError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(config.storage.pool_size)
            .connect(&format!("sqlite:{}?mode=rwc", path))
            .await
            .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        // Enable WAL mode for better concurrent read performance
        sqlx::query("PRAGMA journal_mode=WAL")
            .execute(&pool)
            .await
            .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        let this = Self {
            pool,
            #[cfg(feature = "vector-memory")]
            embed: None,
            config,
        };

        this.run_migrations().await?;

        Ok(this)
    }

    /// Attach an embedding model for vector search.
    #[cfg(feature = "vector-memory")]
    pub fn with_embedding(mut self, embed: Arc<dyn xz_embed::traits::EmbeddingModel>) -> Self {
        self.embed = Some(embed);
        self
    }

    async fn run_migrations(&self) -> Result<(), MemoryError> {
        for stmt in DDL {
            sqlx::query(stmt)
                .execute(&self.pool)
                .await
                .map_err(|e| MemoryError::database_with_source(format!("Migration failed: {}", e), e))?;
        }
        for stmt in FTS_TRIGGERS {
            // FTS triggers may fail if already exist — ignore
            let _ = sqlx::query(stmt).execute(&self.pool).await;
        }
        debug!("sqlite schema migrations complete");
        Ok(())
    }

    /// Get the next sequence number for a session.
    async fn next_seq(&self, session_id: &str) -> Result<u64, MemoryError> {
        let result: Option<(i64,)> = sqlx::query_as(
            "SELECT MAX(seq) FROM messages WHERE session_id = ?",
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        Ok(result.map_or(0, |(max,)| (max + 1) as u64))
    }
}

#[async_trait::async_trait]
#[allow(unused_variables)]
impl MemorySystem for SqliteMemory {
    // === Short-term Memory ===

    async fn append_message(&self, session_id: &str, mut msg: Message) -> Result<(), MemoryError> {
        let seq = self.next_seq(session_id).await?;
        msg.seq = seq;

        let role_str = msg.role.as_str();
        sqlx::query(
            "INSERT OR REPLACE INTO messages (id, session_id, user_id, role, content, token_count, created_at, seq)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&msg.id)
        .bind(&msg.session_id)
        .bind(&msg.user_id)
        .bind(role_str)
        .bind(&msg.content)
        .bind(msg.token_count as i64)
        .bind(msg.created_at as i64)
        .bind(msg.seq as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        info!(session_id = %session_id, seq = %seq, "short_term append");
        Ok(())
    }

    async fn get_recent_messages(
        &self,
        session_id: &str,
        n: usize,
    ) -> Result<Vec<Message>, MemoryError> {
        let rows: Vec<MessageRow> = sqlx::query_as(
            "SELECT id, session_id, user_id, role, content, token_count, created_at, seq
             FROM messages WHERE session_id = ? ORDER BY seq DESC LIMIT ?",
        )
        .bind(session_id)
        .bind(n as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        // Reverse to return in chronological order
        let messages: Vec<Message> = rows.into_iter().rev().map(|r| r.into()).collect();
        Ok(messages)
    }

    async fn get_session_messages(
        &self,
        session_id: &str,
        page: PageRequest,
    ) -> Result<MessagePage, MemoryError> {
        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM messages WHERE session_id = ?",
        )
        .bind(session_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        let total = total.0 as usize;
        let rows: Vec<MessageRow> = sqlx::query_as(
            "SELECT id, session_id, user_id, role, content, token_count, created_at, seq
             FROM messages WHERE session_id = ? ORDER BY seq ASC LIMIT ? OFFSET ?",
        )
        .bind(session_id)
        .bind(page.limit as i64)
        .bind(page.offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        let items: Vec<Message> = rows.into_iter().map(|r| r.into()).collect();
        let has_more = page.offset + page.limit < total;

        Ok(MessagePage {
            items,
            total,
            has_more,
        })
    }

    async fn clear_short_term(&self, session_id: &str) -> Result<(), MemoryError> {
        sqlx::query("DELETE FROM messages WHERE session_id = ?")
            .bind(session_id)
            .execute(&self.pool)
            .await
            .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        info!(session_id = %session_id, "short_term cleared");
        Ok(())
    }

    async fn evict_oldest_messages(
        &self,
        session_id: &str,
        keep_count: usize,
    ) -> Result<usize, MemoryError> {
        // Count total messages for this session
        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM messages WHERE session_id = ?",
        )
        .bind(session_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        let total = total.0 as usize;
        if total <= keep_count {
            return Ok(0);
        }

        let excess = total - keep_count;
        // Delete the oldest `excess` messages (lowest seq)
        let result = sqlx::query(
            "DELETE FROM messages WHERE id IN (
                SELECT id FROM messages WHERE session_id = ? ORDER BY seq ASC LIMIT ?
            )",
        )
        .bind(session_id)
        .bind(excess as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        let evicted = result.rows_affected() as usize;
        info!(session_id = %session_id, evicted, keep = keep_count, "oldest messages evicted");
        Ok(evicted)
    }

    // === Summary Memory ===

    #[cfg(feature = "summary")]
    async fn get_or_create_summary(
        &self,
        session_id: &str,
        provider: &dyn xz_provider::traits::LlmProvider,
    ) -> Result<SessionSummary, MemoryError> {
        // Check if summary already exists
        let existing: Option<SummaryRow> = sqlx::query_as(
            "SELECT session_id, user_id, summary, key_points_json, token_count, message_count, created_at, updated_at
             FROM session_summaries WHERE session_id = ?",
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        if let Some(row) = existing {
            return Ok(row.into());
        }

        // Generate summary from messages using LLM
        let messages = self.get_recent_messages(session_id, 100).await?;
        if messages.is_empty() {
            return Err(MemoryError::SessionNotFound(session_id.to_string()));
        }

        let user_id = messages.first().map(|m| m.user_id.clone()).unwrap_or_default();
        let message_count = messages.len();
        let token_count: usize = messages.iter().map(|m| m.token_count).sum();

        // Build prompt
        let conversation: String = messages
            .iter()
            .map(|m| format!("{}: {}", m.role.as_str(), m.content))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Summarize the following conversation concisely. Include key points as a bullet list.\n\nConversation:\n{}",
            conversation
        );

        let request = xz_provider::CompletionRequest {
            messages: vec![xz_provider::Message::user(&prompt)],
            ..Default::default()
        };

        let options = xz_provider::RequestOptions::default();
        let response = provider
            .complete(request, options)
            .await
            .map_err(|e| MemoryError::SummaryGeneration(e.to_string()))?;

        let summary_text = response.content.unwrap_or_default();

        let summary = SessionSummary {
            session_id: session_id.to_string(),
            user_id,
            summary: summary_text.clone(),
            key_points: vec![],
            token_count,
            message_count,
            created_at: current_epoch_ms(),
            updated_at: current_epoch_ms(),
        };

        self.update_summary(session_id, summary.clone()).await?;
        Ok(summary)
    }

    async fn update_summary(
        &self,
        session_id: &str,
        summary: SessionSummary,
    ) -> Result<(), MemoryError> {
        let key_points_json = serde_json::to_string(&summary.key_points)
            .map_err(|e| MemoryError::serialization_with_source(e.to_string(), e))?;

        sqlx::query(
            "INSERT OR REPLACE INTO session_summaries
             (session_id, user_id, summary, key_points_json, token_count, message_count, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&summary.session_id)
        .bind(&summary.user_id)
        .bind(&summary.summary)
        .bind(&key_points_json)
        .bind(summary.token_count as i64)
        .bind(summary.message_count as i64)
        .bind(summary.created_at as i64)
        .bind(summary.updated_at as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        Ok(())
    }

    async fn get_summary_history(
        &self,
        user_id: &str,
        limit: usize,
    ) -> Result<Vec<SessionSummary>, MemoryError> {
        let rows: Vec<SummaryRow> = sqlx::query_as(
            "SELECT session_id, user_id, summary, key_points_json, token_count, message_count, created_at, updated_at
             FROM session_summaries WHERE user_id = ? ORDER BY updated_at DESC LIMIT ?",
        )
        .bind(user_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    // === Fact Memory ===

    async fn remember_fact(&self, fact: Fact) -> Result<UpsertResult, MemoryError> {
        // Check for existing fact with same (user_id, subject, predicate)
        let existing: Option<FactRow> = sqlx::query_as(
            "SELECT id, user_id, category, subject, predicate, object, confidence, source_session, created_at, updated_at, version
             FROM facts WHERE user_id = ? AND subject = ? AND predicate = ?",
        )
        .bind(&fact.user_id)
        .bind(&fact.subject)
        .bind(&fact.predicate)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        if let Some(row) = existing {
            // Compare fields to detect changes
            let old_fact: Fact = row.into();
            let mut changed_fields = Vec::new();

            if old_fact.object != fact.object {
                changed_fields.push("object".to_string());
            }
            if old_fact.confidence != fact.confidence {
                changed_fields.push("confidence".to_string());
            }
            if old_fact.category != fact.category {
                changed_fields.push("category".to_string());
            }

            if changed_fields.is_empty() {
                return Ok(UpsertResult::Unchanged);
            }

            let category = fact_category_to_str(&fact.category);
            sqlx::query(
                "UPDATE facts SET object = ?, confidence = ?, category = ?, updated_at = ?, version = version + 1
                 WHERE id = ?",
            )
            .bind(&fact.object)
            .bind(fact.confidence.as_f32())
            .bind(category)
            .bind(current_epoch_ms() as i64)
            .bind(&old_fact.id)
            .execute(&self.pool)
            .await
            .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

            return Ok(UpsertResult::Updated { changed_fields });
        }

        // Insert new fact
        let category = fact_category_to_str(&fact.category);
        sqlx::query(
            "INSERT INTO facts (id, user_id, category, subject, predicate, object, confidence, source_session, created_at, updated_at, version)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1)",
        )
        .bind(&fact.id)
        .bind(&fact.user_id)
        .bind(category)
        .bind(&fact.subject)
        .bind(&fact.predicate)
        .bind(&fact.object)
        .bind(fact.confidence.as_f32())
        .bind(&fact.source_session)
        .bind(fact.created_at as i64)
        .bind(fact.updated_at as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        info!(user_id = %fact.user_id, subject = %fact.subject, predicate = %fact.predicate, "fact remembered");
        Ok(UpsertResult::Created)
    }

    async fn recall_facts(
        &self,
        user_id: &str,
        query: &str,
        options: &FactRecallOptions,
    ) -> Result<FactPage, MemoryError> {
        if !self.config.fts.enabled || query.len() < self.config.fts.min_query_length {
            // Fall back to simple LIKE search
            return self.recall_facts_like(user_id, query, options).await;
        }

        self.recall_facts_fts(user_id, query, options).await
    }

    async fn get_user_preferences(&self, user_id: &str) -> Result<Vec<Fact>, MemoryError> {
        let rows: Vec<FactRow> = sqlx::query_as(
            "SELECT id, user_id, category, subject, predicate, object, confidence, source_session, created_at, updated_at, version
             FROM facts WHERE user_id = ? AND category = ?",
        )
        .bind(user_id)
        .bind("Preference")
        .fetch_all(&self.pool)
        .await
        .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn delete_fact(&self, id: &str) -> Result<(), MemoryError> {
        let result = sqlx::query("DELETE FROM facts WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        if result.rows_affected() == 0 {
            return Err(MemoryError::FactNotFound(id.to_string()));
        }

        Ok(())
    }

    async fn compact_facts(
        &self,
        user_id: &str,
        strategy: CompactionStrategy,
    ) -> Result<CompactionResult, MemoryError> {
        let mut result = CompactionResult::default();

        match strategy {
            CompactionStrategy::MergeSimilar => {
                // Find duplicate (user_id, subject, predicate) groups, keep highest confidence
                let duplicates: Vec<DuplicateFact> = sqlx::query_as(
                    "SELECT subject, predicate, COUNT(*) as cnt, MAX(confidence) as max_conf
                     FROM facts WHERE user_id = ? GROUP BY user_id, subject, predicate HAVING cnt > 1",
                )
                .bind(user_id)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

                for dup in &duplicates {
                    let affected = sqlx::query(
                        "DELETE FROM facts WHERE user_id = ? AND subject = ? AND predicate = ? AND confidence < ?",
                    )
                    .bind(user_id)
                    .bind(&dup.subject)
                    .bind(&dup.predicate)
                    .bind(dup.max_conf)
                    .execute(&self.pool)
                    .await
                    .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

                    result.facts_merged += affected.rows_affected() as usize;
                }
            }
            CompactionStrategy::RemoveLowConfidence(threshold) => {
                let affected = sqlx::query(
                    "DELETE FROM facts WHERE user_id = ? AND confidence < ?",
                )
                .bind(user_id)
                .bind(threshold)
                .execute(&self.pool)
                .await
                .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

                result.facts_removed = affected.rows_affected() as usize;
            }
            CompactionStrategy::RemoveOld(before_ts) => {
                let affected = sqlx::query(
                    "DELETE FROM facts WHERE user_id = ? AND updated_at < ? AND confidence < 0.6",
                )
                .bind(user_id)
                .bind(before_ts as i64)
                .execute(&self.pool)
                .await
                .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

                result.facts_removed = affected.rows_affected() as usize;
            }
        }

        // Count remaining facts
        let kept: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM facts WHERE user_id = ?")
            .bind(user_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;
        result.facts_kept = kept.0 as usize;

        warn!(user_id = %user_id, merged = %result.facts_merged, removed = %result.facts_removed, kept = %result.facts_kept, "compaction completed");
        Ok(result)
    }

    // === Vector Memory ===

    #[cfg(feature = "vector-memory")]
    async fn store_vector(&self, entry: crate::types::vector::VectorEntry) -> Result<(), MemoryError> {
        let embedding_blob = bincode_serialize(&entry.vector)
            .map_err(|e| MemoryError::serialization_with_source(e.to_string(), e))?;
        let metadata_json = serde_json::to_string(&entry.metadata)
            .map_err(|e| MemoryError::serialization_with_source(e.to_string(), e))?;

        sqlx::query(
            "INSERT OR REPLACE INTO vectors (id, user_id, content, embedding, metadata_json, created_at, dimension, expires_at, channel)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&entry.id)
        .bind(entry.metadata.get("user_id").map(|s| s.as_str()).unwrap_or(""))
        .bind(&entry.content)
        .bind(&embedding_blob)
        .bind(&metadata_json)
        .bind(entry.created_at as i64)
        .bind(entry.vector.len() as i64)
        .bind(entry.expires_at.map(|t| t as i64))
        .bind(&entry.channel)
        .execute(&self.pool)
        .await
        .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        Ok(())
    }

    #[cfg(feature = "vector-memory")]
    async fn search_vector(
        &self,
        query: &[f32],
        limit: usize,
        threshold: f32,
    ) -> Result<Vec<crate::types::vector::SearchResult>, MemoryError> {
        let rows: Vec<VectorRow> = sqlx::query_as(
            "SELECT id, user_id, content, embedding, metadata_json, created_at, dimension, expires_at, channel FROM vectors",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        let mut results: Vec<crate::types::vector::SearchResult> = Vec::new();

        for row in rows {
            let stored: Vec<f32> = bincode_deserialize(&row.embedding)
                .map_err(|e| MemoryError::serialization_with_source(e.to_string(), e))?;

            if stored.len() != query.len() {
                continue;
            }

            let similarity = cosine_similarity(query, &stored);
            if similarity >= threshold {
                let metadata: std::collections::HashMap<String, String> =
                    serde_json::from_str(&row.metadata_json).unwrap_or_default();

                results.push(crate::types::vector::SearchResult {
                    id: row.id,
                    score: similarity,
                    metadata,
                    content: row.content,
                    channel: None,
                });
            }
        }

        // Sort by score descending, take top N
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);

        Ok(results)
    }

    #[cfg(feature = "vector-memory")]
    async fn delete_vector(&self, id: &str) -> Result<(), MemoryError> {
        let result = sqlx::query("DELETE FROM vectors WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        if result.rows_affected() == 0 {
            return Err(MemoryError::VectorNotFound(id.to_string()));
        }

        Ok(())
    }

    // === Maintenance ===

    async fn stats(&self, user_id: &str) -> Result<MemoryStats, MemoryError> {
        let sessions: (i64,) =
            sqlx::query_as("SELECT COUNT(DISTINCT session_id) FROM messages WHERE user_id = ?")
                .bind(user_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        let messages: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM messages WHERE user_id = ?")
                .bind(user_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        let facts: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM facts WHERE user_id = ?")
                .bind(user_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        let tokens: (i64,) =
            sqlx::query_as("SELECT COALESCE(SUM(token_count), 0) FROM messages WHERE user_id = ?")
                .bind(user_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        #[cfg(feature = "vector-memory")]
        let vectors: (i64,) = {
            sqlx::query_as("SELECT COUNT(*) FROM vectors WHERE user_id = ?")
                .bind(user_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?
        };
        #[cfg(not(feature = "vector-memory"))]
        let vectors: (i64,) = (0,);

        // Approximate DB size
        let db_size: (i64,) = sqlx::query_as("SELECT COALESCE(SUM(pgsize), 0) FROM dbstat")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        Ok(MemoryStats {
            total_sessions: sessions.0 as usize,
            total_messages: messages.0 as usize,
            total_facts: facts.0 as usize,
            total_vectors: vectors.0 as usize,
            total_tokens_approx: tokens.0 as usize,
            db_size_bytes: db_size.0 as u64,
        })
    }

    async fn export(&self, user_id: &str) -> Result<MemoryExport, MemoryError> {
        // Export messages grouped by session
        let message_rows: Vec<MessageRow> = sqlx::query_as(
            "SELECT id, session_id, user_id, role, content, token_count, created_at, seq
             FROM messages WHERE user_id = ? ORDER BY session_id, seq",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        let mut sessions_map: std::collections::HashMap<String, Vec<Message>> =
            std::collections::HashMap::new();
        for row in message_rows {
            let msg: Message = row.into();
            sessions_map
                .entry(msg.session_id.clone())
                .or_default()
                .push(msg);
        }

        // Export summaries
        let summary_rows: Vec<SummaryRow> = sqlx::query_as(
            "SELECT session_id, user_id, summary, key_points_json, token_count, message_count, created_at, updated_at
             FROM session_summaries WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        let summaries: std::collections::HashMap<String, SessionSummary> = summary_rows
            .into_iter()
            .map(|r| {
                let s: SessionSummary = r.into();
                (s.session_id.clone(), s)
            })
            .collect();

        let sessions: Vec<SessionSnapshot> = sessions_map
            .into_iter()
            .map(|(sid, messages)| {
                let summary = summaries.get(&sid).cloned();
                SessionSnapshot {
                    session_id: sid,
                    messages,
                    summary,
                }
            })
            .collect();

        // Export facts
        let fact_rows: Vec<FactRow> = sqlx::query_as(
            "SELECT id, user_id, category, subject, predicate, object, confidence, source_session, created_at, updated_at, version
             FROM facts WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        let facts: Vec<Fact> = fact_rows.into_iter().map(|r| r.into()).collect();

        // Export vectors
        #[allow(unused_mut)]
        let mut vectors: Vec<crate::types::vector::VectorEntry> = vec![];
        #[cfg(feature = "vector-memory")]
        {
            let vector_rows: Vec<VectorRow> = sqlx::query_as(
                "SELECT id, user_id, content, embedding, metadata_json, created_at, dimension, expires_at, channel FROM vectors WHERE user_id = ?",
            )
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

            for row in vector_rows {
                let vector: Vec<f32> = bincode_deserialize(&row.embedding)
                    .map_err(|e| MemoryError::serialization_with_source(e.to_string(), e))?;
                let metadata: std::collections::HashMap<String, String> =
                    serde_json::from_str(&row.metadata_json).unwrap_or_default();

                vectors.push(crate::types::vector::VectorEntry {
                    id: row.id,
                    vector,
                    metadata,
                    content: row.content,
                    created_at: row.created_at as u64,
                    expires_at: row.expires_at.map(|t| t as u64),
                    channel: row.channel,
                });
            }
        }

        Ok(MemoryExport {
            version: "1.0".into(),
            user_id: user_id.to_string(),
            exported_at: current_epoch_ms(),
            sessions,
            facts,
            vectors,
        })
    }

    async fn import(&self, data: MemoryExport) -> Result<ImportResult, MemoryError> {
        let mut result = ImportResult {
            sessions_imported: 0,
            facts_imported: 0,
            vectors_imported: 0,
        };

        for snapshot in &data.sessions {
            for msg in &snapshot.messages {
                self.append_message(&snapshot.session_id, msg.clone())
                    .await?;
            }
            if let Some(ref summary) = snapshot.summary {
                self.update_summary(&snapshot.session_id, summary.clone())
                    .await?;
            }
            result.sessions_imported += 1;
        }

        for fact in &data.facts {
            self.remember_fact(fact.clone()).await?;
            result.facts_imported += 1;
        }

        #[cfg(feature = "vector-memory")]
        for vector in &data.vectors {
            self.store_vector(vector.clone()).await?;
            result.vectors_imported += 1;
        }

        Ok(result)
    }
}

// === Internal helpers ===

impl SqliteMemory {
    async fn recall_facts_fts(
        &self,
        user_id: &str,
        query: &str,
        options: &FactRecallOptions,
    ) -> Result<FactPage, MemoryError> {
        let fts_query = format!("\"{}\"", query.replace('"', "\"\""));
        let limit = options.page.limit as i64;
        let offset = options.page.offset as i64;

        // Count total matches
        let count_sql = format!(
            "SELECT COUNT(*) FROM facts f
             JOIN facts_fts fts ON f.rowid = fts.rowid
             WHERE f.user_id = ? AND facts_fts MATCH '{}'",
            fts_query
        );
        let total: (i64,) = sqlx::query_as(&count_sql)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        // Fetch matching facts
        let select_sql = format!(
            "SELECT f.id, f.user_id, f.category, f.subject, f.predicate, f.object, f.confidence,
                    f.source_session, f.created_at, f.updated_at, f.version
             FROM facts f
             JOIN facts_fts fts ON f.rowid = fts.rowid
             WHERE f.user_id = ? AND facts_fts MATCH '{}'
             ORDER BY f.updated_at DESC
             LIMIT ? OFFSET ?",
            fts_query
        );

        let rows: Vec<FactRow> = sqlx::query_as(&select_sql)
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        let total = total.0 as usize;
        let items: Vec<Fact> = rows.into_iter().map(|r| r.into()).collect();
        let has_more = (offset as usize + limit as usize) < total;

        Ok(FactPage {
            items,
            total,
            has_more,
        })
    }

    async fn recall_facts_like(
        &self,
        user_id: &str,
        query: &str,
        options: &FactRecallOptions,
    ) -> Result<FactPage, MemoryError> {
        let like = format!("%{}%", query);
        let limit = options.page.limit as i64;
        let offset = options.page.offset as i64;

        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM facts WHERE user_id = ? AND (subject LIKE ? OR predicate LIKE ? OR object LIKE ?)",
        )
        .bind(user_id)
        .bind(&like)
        .bind(&like)
        .bind(&like)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        let rows: Vec<FactRow> = sqlx::query_as(
            "SELECT id, user_id, category, subject, predicate, object, confidence, source_session, created_at, updated_at, version
             FROM facts WHERE user_id = ? AND (subject LIKE ? OR predicate LIKE ? OR object LIKE ?)
             ORDER BY updated_at DESC LIMIT ? OFFSET ?",
        )
        .bind(user_id)
        .bind(&like)
        .bind(&like)
        .bind(&like)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        let total = total.0 as usize;
        let items: Vec<Fact> = rows.into_iter().map(|r| r.into()).collect();
        let has_more = (offset as usize + limit as usize) < total;

        Ok(FactPage {
            items,
            total,
            has_more,
        })
    }
}

// === Row types for sqlx mapping ===

#[derive(Debug, sqlx::FromRow)]
struct MessageRow {
    id: String,
    session_id: String,
    user_id: String,
    role: String,
    content: String,
    token_count: i64,
    created_at: i64,
    seq: i64,
}

impl From<MessageRow> for Message {
    fn from(r: MessageRow) -> Self {
        Self {
            id: r.id,
            session_id: r.session_id,
            user_id: r.user_id,
            role: match r.role.as_str() {
                "system" => Role::System,
                "assistant" => Role::Assistant,
                "tool" => Role::Tool,
                _ => Role::User,
            },
            content: r.content,
            token_count: r.token_count as usize,
            created_at: r.created_at as u64,
            seq: r.seq as u64,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct SummaryRow {
    session_id: String,
    user_id: String,
    summary: String,
    key_points_json: String,
    token_count: i64,
    message_count: i64,
    created_at: i64,
    updated_at: i64,
}

impl From<SummaryRow> for SessionSummary {
    fn from(r: SummaryRow) -> Self {
        let key_points: Vec<String> =
            serde_json::from_str(&r.key_points_json).unwrap_or_default();
        Self {
            session_id: r.session_id,
            user_id: r.user_id,
            summary: r.summary,
            key_points,
            token_count: r.token_count as usize,
            message_count: r.message_count as usize,
            created_at: r.created_at as u64,
            updated_at: r.updated_at as u64,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct FactRow {
    id: String,
    user_id: String,
    category: String,
    subject: String,
    predicate: String,
    object: String,
    confidence: f32,
    source_session: Option<String>,
    created_at: i64,
    updated_at: i64,
    version: i64,
}

impl From<FactRow> for Fact {
    fn from(r: FactRow) -> Self {
        Self {
            id: r.id,
            user_id: r.user_id,
            category: str_to_fact_category(&r.category),
            subject: r.subject,
            predicate: r.predicate,
            object: r.object,
            confidence: Confidence::from_f32(r.confidence),
            source_session: r.source_session,
            created_at: r.created_at as u64,
            updated_at: r.updated_at as u64,
            version: r.version as u64,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct DuplicateFact {
    subject: String,
    predicate: String,
    #[allow(dead_code)]
    cnt: i64,
    max_conf: f32,
}

#[cfg(feature = "vector-memory")]
#[derive(Debug, sqlx::FromRow)]
struct VectorRow {
    id: String,
    #[allow(dead_code)]
    user_id: String,
    content: Option<String>,
    embedding: Vec<u8>,
    metadata_json: String,
    created_at: i64,
    #[allow(dead_code)]
    dimension: i64,
    expires_at: Option<i64>,
    channel: Option<String>,
}

// === Utility functions ===

fn current_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn fact_category_to_str(cat: &FactCategory) -> &'static str {
    match cat {
        FactCategory::Preference => "Preference",
        FactCategory::PersonalInfo => "PersonalInfo",
        FactCategory::Relationship => "Relationship",
        FactCategory::Event => "Event",
        FactCategory::Schedule => "Schedule",
        FactCategory::Health => "Health",
        FactCategory::Location => "Location",
        FactCategory::Custom(_) => "Custom",
    }
}

fn str_to_fact_category(s: &str) -> FactCategory {
    match s {
        "Preference" => FactCategory::Preference,
        "PersonalInfo" => FactCategory::PersonalInfo,
        "Relationship" => FactCategory::Relationship,
        "Event" => FactCategory::Event,
        "Schedule" => FactCategory::Schedule,
        "Health" => FactCategory::Health,
        "Location" => FactCategory::Location,
        other => FactCategory::Custom(other.to_string()),
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

/// Simple bincode-like serialization for f32 vectors.
#[cfg(feature = "vector-memory")]
fn bincode_serialize(v: &[f32]) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let mut bytes = Vec::with_capacity(v.len() * 4);
    for f in v {
        bytes.extend_from_slice(&f.to_le_bytes());
    }
    Ok(bytes)
}

#[cfg(feature = "vector-memory")]
fn bincode_deserialize(bytes: &[u8]) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>> {
    let floats: Vec<f32> = bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();
    Ok(floats)
}
