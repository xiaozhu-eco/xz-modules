/// Agent scheduler errors.
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("Agent not found: {0}")]
    NotFound(String),

    #[error("Step execution failed: {step} -- {reason}")]
    StepFailed { step: String, reason: String },

    #[error("Scheduler not running")]
    SchedulerNotRunning,

    #[error("Execution timed out ({0}s)")]
    Timeout(u64),

    #[error("Concurrency limit: max {max}")]
    ConcurrencyLimit { max: usize },

    #[error("Circular dependency: {0:?}")]
    CircularDependency(Vec<String>),

    #[error("Paused: {0}")]
    Paused(String),

    #[error("Cancelled: {0}")]
    Cancelled(String),

    #[error("IO error: {0}")]
    Io(String),
}

impl AgentError {
    /// Whether this error is retryable (transient) or permanent.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            AgentError::Timeout(_)
                | AgentError::ConcurrencyLimit { .. }
                | AgentError::SchedulerNotRunning
                | AgentError::Io(_)
        )
    }
}
