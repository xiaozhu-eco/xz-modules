use serde::{Deserialize, Serialize};

/// A single step in an Agent's execution DAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStep {
    pub id: String,
    pub name: String,
    pub action: AgentAction,
    pub depends_on: Vec<String>,
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub retry_backoff_ms: u64,
    pub on_failure: OnFailure,
}

impl AgentStep {
    pub fn new(id: impl Into<String>, name: impl Into<String>, action: AgentAction) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            action,
            depends_on: vec![],
            timeout_secs: 60,
            max_retries: 0,
            retry_backoff_ms: 1000,
            on_failure: OnFailure::Abort,
        }
    }

    pub fn depends_on(mut self, dep: impl Into<String>) -> Self {
        self.depends_on.push(dep.into());
        self
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    pub fn with_retry(mut self, max_retries: u32, backoff_ms: u64) -> Self {
        self.max_retries = max_retries;
        self.retry_backoff_ms = backoff_ms;
        self
    }

    pub fn with_on_failure(mut self, on_failure: OnFailure) -> Self {
        self.on_failure = on_failure;
        self
    }
}

/// Action types that an Agent step can execute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentAction {
    LlmCall {
        prompt_template: String,
        model: String,
        temperature: f32,
        max_tokens: u32,
    },
    WebSearch {
        query_template: String,
        sources: Vec<String>,
        max_results: usize,
    },
    SkillInvoke {
        skill_id: String,
        input_template: String,
    },
    WebExtract {
        url_template: String,
        selector: Option<String>,
    },
    GenerateReport {
        format: ReportFormat,
        template: Option<String>,
    },
    Notify {
        method: NotificationMethod,
        title_template: String,
        body_template: String,
    },
    CodeBlock {
        code: String,
        language: String,
        timeout_secs: u64,
    },
    MemoryRecall {
        query_template: String,
        limit: usize,
    },
    Condition {
        expression: String,
        then: Vec<AgentStep>,
        r#else: Vec<AgentStep>,
    },
}

/// Report output format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReportFormat {
    Markdown,
    Html,
    Json,
    PlainText,
}

/// Notification delivery method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationMethod {
    InApp,
    DesktopNotification,
    Email { to: String },
    Webhook { url: String },
}

/// On-failure behavior for a step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OnFailure {
    Abort,
    Skip,
    Retry,
    Fallback { step_id: String },
}
