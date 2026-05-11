use serde::{Deserialize, Serialize};

use crate::types::agent::AgentTrigger;

/// Agent status variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStatus {
    Idle,
    Running { run_id: String, started_at: u64 },
    Paused,
    Error { msg: String },
}

/// Filter for listing agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentFilter {
    pub enabled_only: bool,
    pub trigger_type: Option<AgentTrigger>,
    pub page: PageRequest,
}

impl Default for AgentFilter {
    fn default() -> Self {
        Self {
            enabled_only: true,
            trigger_type: None,
            page: PageRequest::default(),
        }
    }
}

/// Result of an upsert operation (register/update).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpsertResult {
    Created,
    Updated,
}

/// Pagination request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageRequest {
    pub limit: usize,
    pub offset: usize,
}

impl Default for PageRequest {
    fn default() -> Self {
        Self { limit: 50, offset: 0 }
    }
}
