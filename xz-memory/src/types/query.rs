use serde::{Deserialize, Serialize};

use super::fact::Fact;
use super::message::Message;
use super::session::SessionSnapshot;
use super::vector::VectorEntry;

/// Upsert result for idempotent operations.
#[derive(Debug, Clone)]
pub enum UpsertResult {
    Created,
    Updated { changed_fields: Vec<String> },
    Unchanged,
}

/// Paginated message results.
#[derive(Debug, Clone)]
pub struct MessagePage {
    pub items: Vec<Message>,
    pub total: usize,
    pub has_more: bool,
}

/// Pagination request.
#[derive(Debug, Clone)]
pub struct PageRequest {
    pub limit: usize,
    pub offset: usize,
}

impl Default for PageRequest {
    fn default() -> Self {
        Self { limit: 50, offset: 0 }
    }
}

/// Memory system statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_sessions: usize,
    pub total_messages: usize,
    pub total_facts: usize,
    pub total_vectors: usize,
    pub total_tokens_approx: usize,
    pub db_size_bytes: u64,
}

/// Full memory export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryExport {
    pub version: String,
    pub user_id: String,
    pub exported_at: u64,
    pub sessions: Vec<SessionSnapshot>,
    pub facts: Vec<Fact>,
    pub vectors: Vec<VectorEntry>,
}

/// Import result.
#[derive(Debug, Clone)]
pub struct ImportResult {
    pub sessions_imported: usize,
    pub facts_imported: usize,
    pub vectors_imported: usize,
}
