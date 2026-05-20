/// Agent scheduler errors.
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    /// Agent with the given identifier could not be found.
    #[error("Agent not found: {0}")]
    NotFound(String),

    /// A step within an agent sequence failed.
    #[error("Step execution failed: {step} -- {reason}")]
    StepFailed {
        /// Name of the step that failed.
        step: String,
        /// Reason for failure.
        reason: String,
    },

    /// The scheduler is not in a running state.
    #[error("Scheduler not running")]
    SchedulerNotRunning,

    /// Execution exceeded the configured timeout.
    #[error("Execution timed out ({0}s)")]
    Timeout(u64),

    /// Maximum concurrency limit has been reached.
    #[error("Concurrency limit: max {max}")]
    ConcurrencyLimit {
        /// Maximum concurrent executions allowed.
        max: usize,
    },

    /// A circular dependency was detected in the DAG.
    #[error("Circular dependency detected: {0:?}")]
    CircularDependency(Vec<String>),

    /// Execution failed with a potentially transient error.
    #[error("Agent execution failed: {0}")]
    ExecutionFailed(String),

    /// Scheduler is paused.
    #[error("Paused: {0}")]
    Paused(String),

    /// Execution was cancelled.
    #[error("Cancelled: {0}")]
    Cancelled(String),

    /// An I/O level error occurred.
    #[error("IO error: {0}")]
    Io(String),
}

impl AgentError {
    /// Returns `true` if the error is transient and the operation may be
    /// retried, or `false` if the error is permanent.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            AgentError::Timeout(_)
                | AgentError::ConcurrencyLimit { .. }
                | AgentError::SchedulerNotRunning
                | AgentError::Io(_)
                | AgentError::ExecutionFailed(_)
        )
    }
}
