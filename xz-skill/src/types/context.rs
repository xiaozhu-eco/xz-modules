use std::sync::Arc;

/// Execution context passed to skill runtime — provides access to LLM, search, memory, etc.
#[derive(Clone)]
pub struct ExecutionContext {
    pub user_id: String,
    pub session_id: String,
    pub messages: Vec<Message>,
    pub provider: Option<Arc<dyn xz_provider::LlmProvider>>,
    pub search: Option<Arc<dyn std::any::Any + Send + Sync>>,
    pub memory: Option<Arc<dyn std::any::Any + Send + Sync>>,
}

impl std::fmt::Debug for ExecutionContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecutionContext")
            .field("user_id", &self.user_id)
            .field("session_id", &self.session_id)
            .field("messages", &self.messages)
            .field("provider", &self.provider.is_some())
            .field("search", &self.search.is_some())
            .field("memory", &self.memory.is_some())
            .finish()
    }
}

/// A simple chat message in the execution context.
#[derive(Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Default for Message {
    fn default() -> Self {
        Self {
            role: "user".into(),
            content: String::new(),
        }
    }
}
