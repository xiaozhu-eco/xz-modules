use async_trait::async_trait;
use std::fmt::Debug;

use crate::error::AgentError;
use crate::types::agent::Agent;
use crate::types::result::AgentRunResult;
use crate::types::status::{AgentFilter, AgentStatus, UpsertResult};

/// Agent scheduler trait.
///
/// Manages agent registration, triggering, lifecycle (start/stop/pause/resume),
/// and status tracking.
#[async_trait]
pub trait AgentScheduler: Send + Sync + Debug {
    /// Register a new agent or update an existing one.
    async fn register(&self, agent: Agent) -> Result<UpsertResult, AgentError>;

    /// Unregister an agent by ID.
    async fn unregister(&self, id: &str) -> Result<(), AgentError>;

    /// Manually trigger an agent run.
    async fn trigger(&self, id: &str, input: Option<&str>) -> Result<AgentRunResult, AgentError>;

    /// Trigger multiple agents in batch.
    async fn trigger_batch(
        &self,
        ids: &[&str],
    ) -> Result<Vec<AgentRunResult>, AgentError>;

    /// Start the scheduler (auto-triggers based on cron/interval/event).
    async fn start(&self) -> Result<(), AgentError>;

    /// Stop the scheduler.
    async fn stop(&self) -> Result<(), AgentError>;

    /// List agents matching the filter.
    async fn list(&self, filter: &AgentFilter) -> Result<Vec<Agent>, AgentError>;

    /// Get current status of an agent.
    async fn get_status(&self, id: &str) -> Result<AgentStatus, AgentError>;

    /// Cancel a running agent run.
    async fn cancel(&self, run_id: &str) -> Result<(), AgentError>;

    /// Pause an agent (prevent auto-triggering).
    async fn pause(&self, id: &str) -> Result<(), AgentError>;

    /// Resume a paused agent.
    async fn resume(&self, id: &str) -> Result<(), AgentError>;
}
