use async_trait::async_trait;
use std::fmt::Debug;

use crate::error::MemoryError;
use crate::types::fact::{CompactionResult, CompactionStrategy, Fact, FactPage, FactRecallOptions};
use crate::types::message::Message;
use crate::types::query::{ImportResult, MemoryExport, MemoryStats, MessagePage, PageRequest, UpsertResult};
use crate::types::session::SessionSummary;

/// Layered memory system core interface.
///
/// Design principles:
/// - Each memory layer has independent operations (short-term/summary/fact/vector)
/// - Upsert semantics by ID/session_id
/// - Memory compaction is a first-class operation (LLM-driven summarization)
/// - Search returns paginated results with sorting
/// - Forgetting is explicit (policy-based expiry/eviction)
#[async_trait]
pub trait MemorySystem: Send + Sync + Debug {
    // === Short-term Memory ===

    /// Append a message to a session window.
    async fn append_message(&self, session_id: &str, msg: Message) -> Result<(), MemoryError>;

    /// Get the most recent N messages from a session.
    async fn get_recent_messages(
        &self,
        session_id: &str,
        n: usize,
    ) -> Result<Vec<Message>, MemoryError>;

    /// Get full session messages with pagination.
    async fn get_session_messages(
        &self,
        session_id: &str,
        page: PageRequest,
    ) -> Result<MessagePage, MemoryError>;

    /// Clear short-term memory (retains summary).
    async fn clear_short_term(&self, session_id: &str) -> Result<(), MemoryError>;

    /// Evict the oldest messages, keeping only `keep_count` most recent.
    /// Returns the number of messages evicted.
    async fn evict_oldest_messages(
        &self,
        session_id: &str,
        keep_count: usize,
    ) -> Result<usize, MemoryError>;

    // === Summary Memory ===

    /// Get or generate a session summary (lazy loading).
    #[cfg(feature = "summary")]
    async fn get_or_create_summary(
        &self,
        session_id: &str,
        provider: &dyn xz_provider::LlmProvider,
    ) -> Result<SessionSummary, MemoryError>;

    /// Incrementally update a session summary after appending new messages.
    async fn update_summary(
        &self,
        session_id: &str,
        summary: SessionSummary,
    ) -> Result<(), MemoryError>;

    /// Get summary history across sessions.
    async fn get_summary_history(
        &self,
        user_id: &str,
        limit: usize,
    ) -> Result<Vec<SessionSummary>, MemoryError>;

    // === Fact Memory ===

    /// Remember a fact with upsert semantics (dedup by user_id + subject + predicate).
    async fn remember_fact(&self, fact: Fact) -> Result<UpsertResult, MemoryError>;

    /// Search facts via FTS5 + optionally vector hybrid.
    async fn recall_facts(
        &self,
        user_id: &str,
        query: &str,
        options: &FactRecallOptions,
    ) -> Result<FactPage, MemoryError>;

    /// Get user preferences (facts with category = Preference).
    async fn get_user_preferences(&self, user_id: &str) -> Result<Vec<Fact>, MemoryError>;

    /// Delete a fact by ID.
    async fn delete_fact(&self, id: &str) -> Result<(), MemoryError>;

    /// Compact facts: merge similar or remove low-confidence.
    async fn compact_facts(
        &self,
        user_id: &str,
        strategy: CompactionStrategy,
    ) -> Result<CompactionResult, MemoryError>;

    // === Vector Memory (feature-gated) ===

    #[cfg(feature = "vector-memory")]
    async fn store_vector(&self, entry: crate::types::vector::VectorEntry) -> Result<(), MemoryError>;

    #[cfg(feature = "vector-memory")]
    async fn search_vector(
        &self,
        query: &[f32],
        limit: usize,
        threshold: f32,
    ) -> Result<Vec<crate::types::vector::SearchResult>, MemoryError>;

    #[cfg(feature = "vector-memory")]
    async fn delete_vector(&self, id: &str) -> Result<(), MemoryError>;

    // === Maintenance ===

    /// Get memory statistics for a user.
    async fn stats(&self, user_id: &str) -> Result<MemoryStats, MemoryError>;

    /// Full export for backup/migration.
    async fn export(&self, user_id: &str) -> Result<MemoryExport, MemoryError>;

    /// Full import from a previously exported snapshot.
    async fn import(&self, data: MemoryExport) -> Result<ImportResult, MemoryError>;
}
