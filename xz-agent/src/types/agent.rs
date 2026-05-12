use serde::{Deserialize, Serialize};

use super::step::AgentStep;

/// Agent definition with trigger, steps, and configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub description: String,
    pub trigger: AgentTrigger,
    pub steps: Vec<AgentStep>,
    pub config: AgentConfig,
    pub enabled: bool,
    pub created_at: u64,
    pub updated_at: u64,
    pub version: u64,
}

impl Agent {
    pub fn new(id: impl Into<String>, name: impl Into<String>, trigger: AgentTrigger) -> Self {
        let now = current_epoch_ms();
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            trigger,
            steps: vec![],
            config: AgentConfig::default(),
            enabled: true,
            created_at: now,
            updated_at: now,
            version: 1,
        }
    }

    pub fn with_steps(mut self, steps: Vec<AgentStep>) -> Self {
        self.steps = steps;
        self
    }

    pub fn with_config(mut self, config: AgentConfig) -> Self {
        self.config = config;
        self
    }
}

/// Agent trigger types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentTrigger {
    Cron {
        expression: String,
        timezone: String,
    },
    Interval {
        seconds: u64,
    },
    Event {
        event_type: String,
        filter: Option<serde_json::Value>,
    },
    Manual,
    Webhook {
        path: String,
        secret: Option<String>,
    },
}

impl AgentTrigger {
    pub fn cron(expression: impl Into<String>, timezone: impl Into<String>) -> Self {
        Self::Cron {
            expression: expression.into(),
            timezone: timezone.into(),
        }
    }

    pub fn interval(seconds: u64) -> Self {
        Self::Interval { seconds }
    }

    pub fn manual() -> Self {
        Self::Manual
    }
}

/// Agent execution configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub max_execution_time_secs: u64,
    pub max_token_usage: u32,
    pub output_limit_bytes: usize,
    pub max_concurrent_runs: usize,
    pub retry_limit: u32,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_execution_time_secs: 300,
            max_token_usage: 10000,
            output_limit_bytes: 1024 * 1024,
            max_concurrent_runs: 1,
            retry_limit: 3,
        }
    }
}

fn current_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
