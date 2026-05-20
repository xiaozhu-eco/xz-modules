//! Agent definition and result types for DAG scheduling.

/// A lightweight task-definition node for DAG-based agent scheduling.
///
/// Each [`AgentDef`] carries a logical name, a natural-language task
/// description, and an optional list of dependency names that must
/// complete before this agent can execute.
#[derive(Debug, Clone)]
pub struct AgentDef {
    /// Unique logical name for this agent node within the DAG.
    pub name: String,
    /// Natural-language description of the work this agent performs.
    pub task: String,
    /// Names of other agents that must complete before this one runs.
    pub depends_on: Vec<String>,
}

/// A simplified execution result for a single DAG agent node.
///
/// This is distinct from the richer [`crate::types::result::AgentRunResult`]
/// which tracks per-step details, token usage, and wall-clock timestamps.
/// The DAG-scheduler variant is intentionally minimal so dependency checks
/// remain lightweight.
#[derive(Debug, Clone)]
pub struct AgentRunResult {
    /// Name of the agent that produced this result (matches [`AgentDef::name`]).
    pub agent_name: String,
    /// Free-form output text produced during agent execution.
    pub output: String,
    /// Whether the agent completed without errors.
    pub success: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AgentError;

    #[test]
    fn agent_def_creation() {
        // Verify that an AgentDef can be created and all fields are
        // accessible with the expected values.
        let def = AgentDef {
            name: "test".into(),
            task: "do_something".into(),
            depends_on: vec!["dep1".into()],
        };
        assert_eq!(def.name, "test");
        assert_eq!(def.task, "do_something");
        assert_eq!(def.depends_on, vec!["dep1"]);
    }

    #[test]
    fn agent_error_is_retryable() {
        // Verify that AgentError::CircularDependency is NOT retryable,
        // AgentError::ExecutionFailed IS retryable, and existing
        // variant behaviour is unchanged.
        let circ = AgentError::CircularDependency(vec!["a".into(), "b".into()]);
        assert!(!circ.is_retryable(), "circular dependency should not be retryable");

        let exec_fail = AgentError::ExecutionFailed("oops".into());
        assert!(exec_fail.is_retryable(), "execution failed should be retryable");

        // Existing variants — untouched behaviour
        assert!(
            AgentError::Timeout(30).is_retryable(),
            "Timeout is retryable"
        );
        assert!(
            !AgentError::NotFound("x".into()).is_retryable(),
            "NotFound is not retryable"
        );
        assert!(
            !AgentError::StepFailed {
                step: "s".into(),
                reason: "r".into()
            }
            .is_retryable(),
            "StepFailed is not retryable"
        );
    }

    #[test]
    fn agent_run_result_success_flag() {
        // Verify that an AgentRunResult can be created with success: false
        // and that all fields are correctly initialised.
        let result = AgentRunResult {
            agent_name: "test-agent".into(),
            output: "error".into(),
            success: false,
        };
        assert!(!result.success);
        assert_eq!(result.agent_name, "test-agent");
        assert_eq!(result.output, "error");
    }
}
