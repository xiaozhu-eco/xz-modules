use serde::{Deserialize, Serialize};

/// Agent run result containing step-level details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRunResult {
    pub run_id: String,
    pub agent_id: String,
    pub success: bool,
    pub started_at: u64,
    pub completed_at: u64,
    pub output: Option<String>,
    pub error: Option<String>,
    pub steps_completed: Vec<String>,
    pub steps_failed: Vec<String>,
    pub token_usage: TokenUsage,
    pub step_results: Vec<StepResult>,
}

/// Result of a single step execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_id: String,
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub retries: u32,
}

impl StepResult {
    pub fn success(step_id: impl Into<String>, output: Option<String>, duration_ms: u64) -> Self {
        Self {
            step_id: step_id.into(),
            success: true,
            output,
            error: None,
            duration_ms,
            retries: 0,
        }
    }

    pub fn failure(
        step_id: impl Into<String>,
        error: impl Into<String>,
        duration_ms: u64,
        retries: u32,
    ) -> Self {
        Self {
            step_id: step_id.into(),
            success: false,
            output: None,
            error: Some(error.into()),
            duration_ms,
            retries,
        }
    }
}

/// Token usage tracking.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}
