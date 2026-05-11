use serde::{Deserialize, Serialize};

/// Output of a skill execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOutput {
    pub content: String,
    pub tool_calls: Vec<ToolCallRecord>,
    pub token_usage: TokenUsage,
    pub duration_ms: u64,
}

/// Record of a single tool invocation during execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    pub tool_name: String,
    pub args: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

/// Token usage summary.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

/// Lightweight skill summary for list/search results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSummary {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub enabled: bool,
    pub tool_count: usize,
}
