use serde::{Deserialize, Serialize};

/// Scheduler runtime configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    /// Maximum concurrent agent runs across all agents.
    pub global_max_concurrent: usize,
    /// Default timeout for step execution in seconds.
    pub default_step_timeout_secs: u64,
    /// Maximum depth of agent chain (A->B->C->...).
    pub max_chain_depth: usize,
    /// Enable auto-triggering (cron/interval/event).
    pub auto_trigger_enabled: bool,
    /// Global token budget across all agents.
    pub global_token_budget: u64,
    /// LLM rate limit (requests per second).
    pub rate_limit_rps: f64,
    /// WASM fuel metering limit for CodeBlock actions.
    pub wasm_fuel: u64,
    /// Maximum output size in bytes for any single action.
    pub output_size_limit_bytes: usize,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            global_max_concurrent: 10,
            default_step_timeout_secs: 300,
            max_chain_depth: 5,
            auto_trigger_enabled: true,
            global_token_budget: 1_000_000,
            rate_limit_rps: 10.0,
            wasm_fuel: 10_000_000,
            output_size_limit_bytes: 1024 * 1024,
        }
    }
}
