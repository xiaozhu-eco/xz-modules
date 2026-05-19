//! Conversation management for LLM-based autonomous agents.
//!
//! [`ConversationManager`] manages the message flow between an agent and an LLM
//! provider, supporting tool result injection and automatic context compression
//! when the message history exceeds configurable limits.
//!
//! # Example
//!
//! ```rust,no_run
//! use xz_agent::conversation::{ConversationManager, ConversationConfig};
//!
//! let mut manager = ConversationManager::new(ConversationConfig::default());
//! manager.start(
//!     "You are a helpful assistant.".to_string(),
//!     "Hello!".to_string(),
//! );
//! assert_eq!(manager.count_messages(), 2);
//! ```

use xz_provider::{
    CompletionRequest, CompletionResponse, LlmProvider, Message, RequestOptions, ToolCall,
    ToolDefinition,
};

use crate::error::AgentError;

// ── ConversationConfig ──

/// Configuration for [`ConversationManager`].
///
/// Controls model selection, token limits, temperature, and when context
/// compression kicks in.
#[derive(Debug, Clone)]
pub struct ConversationConfig {
    /// Model name passed to the provider (e.g. `"gpt-4o"`).
    pub model: String,
    /// Maximum completion tokens (sent as `max_tokens` in the request).
    pub max_tokens: Option<usize>,
    /// Sampling temperature.
    pub temperature: Option<f32>,
    /// Maximum number of messages before compression is triggered.
    ///
    /// When the message count exceeds this value and
    /// [`compression_enabled`](Self::compression_enabled) is `true`,
    /// [`ConversationManager::maybe_compress`] will attempt to summarise
    /// the oldest messages.
    pub max_context_messages: usize,
    /// Whether automatic context compression is enabled.
    pub compression_enabled: bool,
}

impl Default for ConversationConfig {
    fn default() -> Self {
        Self {
            model: "gpt-4o".to_string(),
            max_tokens: None,
            temperature: None,
            max_context_messages: 50,
            compression_enabled: true,
        }
    }
}

// ── ConversationResponse ──

/// Outcome of a single LLM turn within a conversation.
#[derive(Debug, Clone)]
pub enum ConversationResponse {
    /// The LLM returned a plain text response.
    Text {
        /// Textual content produced by the model.
        content: String,
    },
    /// The LLM requested tool invocations.
    ToolCalls {
        /// Collection of tool calls to execute.
        calls: Vec<ToolCall>,
    },
}

// ── ConversationManager ──

/// Stateful manager for a multi-turn conversation with an LLM.
///
/// Maintains the complete message history and provides facilities to:
/// - Start a conversation with a system prompt and user message
/// - Obtain the next LLM response (optionally with tool definitions)
/// - Inject tool execution results back into the history
/// - Compress the history when it grows too large
#[derive(Debug, Clone)]
pub struct ConversationManager {
    messages: Vec<Message>,
    config: ConversationConfig,
}

impl ConversationManager {
    /// Create a new manager with the given configuration.
    pub fn new(config: ConversationConfig) -> Self {
        Self {
            messages: Vec::new(),
            config,
        }
    }

    /// Initialise the conversation with a system message followed by an
    /// initial user message.
    ///
    /// Any existing messages are replaced.
    pub fn start(&mut self, system_message: String, user_message: String) {
        self.messages.clear();
        self.messages.push(Message::system(&system_message));
        self.messages.push(Message::user(&user_message));
    }

    /// Send the current message history to the LLM provider and return the
    /// response.
    ///
    /// If `tools` is provided, the LLM may elect to request tool calls
    /// instead of producing text.  The returned [`ConversationResponse`]
    /// differentiates the two cases.
    ///
    /// The assistant message (including any tool calls) is automatically
    /// appended to the internal message list.
    pub async fn next_response(
        &mut self,
        provider: &dyn LlmProvider,
        tools: Option<&[ToolDefinition]>,
    ) -> Result<ConversationResponse, AgentError> {
        let mut request = CompletionRequest::new(&self.config.model, self.messages.clone());
        request.temperature = self.config.temperature;
        request.max_tokens = self.config.max_tokens;
        if let Some(t) = tools {
            request.tools = Some(t.to_vec());
        }

        let response = provider
            .complete(request, RequestOptions::default())
            .await
            .map_err(|e| AgentError::Io(format!("LLM provider error: {e}")))?;

        let conversation_response = self.process_completion_response(&response);

        Ok(conversation_response)
    }

    /// Decode a [`CompletionResponse`] into a [`ConversationResponse`] and
    /// record the assistant message in the history.
    fn process_completion_response(
        &mut self,
        response: &CompletionResponse,
    ) -> ConversationResponse {
        let content = response.content.clone().unwrap_or_default();

        if !response.tool_calls.is_empty() {
            let calls = response.tool_calls.clone();
            // Record assistant message with tool calls (empty text content).
            self.messages.push(Message::Assistant {
                content: xz_provider::MessageContent::None,
                tool_calls: Some(calls.clone()),
                cache_control: None,
            });
            ConversationResponse::ToolCalls { calls }
        } else {
            // Record assistant text response.
            self.messages.push(Message::assistant(&content));
            ConversationResponse::Text { content }
        }
    }

    /// Append a tool execution result to the conversation.
    ///
    /// `tool_call_id` must match the `id` of a [`ToolCall`] previously
    /// returned by the LLM.  `is_error` indicates whether the tool execution
    /// failed.
    pub fn inject_tool_result(&mut self, tool_call_id: &str, content: &str, is_error: bool) {
        if is_error {
            self.messages
                .push(Message::tool_error(tool_call_id, content));
        } else {
            self.messages
                .push(Message::tool_result(tool_call_id, content));
        }
    }

    /// Append an arbitrary message to the conversation history.
    pub fn append_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    /// Return a shared reference to the current message history.
    pub fn get_messages(&self) -> &[Message] {
        &self.messages
    }

    /// Attempt to compress the message history if it exceeds the configured
    /// [`ConversationConfig::max_context_messages`] threshold.
    ///
    /// The compression strategy:
    /// 1. Preserve the system message at the front.
    /// 2. Keep the most recent messages (half of `max_context_messages`).
    /// 3. Summarise the removed middle block using an LLM call.
    /// 4. Insert the summary as a user message after the system prompt.
    ///
    /// This is a **best-effort** operation — failures are logged via
    /// [`tracing::warn!`] and do **not** propagate.
    pub async fn maybe_compress(&mut self, provider: &dyn LlmProvider) {
        if !self.config.compression_enabled {
            return;
        }

        let threshold = self.config.max_context_messages;
        if self.messages.len() <= threshold {
            return;
        }

        // Identify the system message index (should be index 0 in normal
        // usage, but we defensively search for it).
        let system_idx = self
            .messages
            .iter()
            .position(|m| matches!(m, Message::System { .. }));

        // Split: [prefix] ... [to_summarize] ... [to_keep]
        let keep_count = threshold / 2;
        let keep_start = self.messages.len().saturating_sub(keep_count);
        let summarize_start = system_idx.map(|i| i + 1).unwrap_or(0);

        if summarize_start >= keep_start {
            // Nothing to summarise — the "old" block is already inside the
            // keep region.
            return;
        }

        let to_summarize: Vec<&Message> = self.messages[summarize_start..keep_start].iter().collect();
        let to_keep: Vec<Message> = self.messages[keep_start..].to_vec();

        match self.summarize_messages(provider, &to_summarize).await {
            Ok(summary) => {
                let mut compressed = Vec::new();
                if let Some(idx) = system_idx {
                    compressed.push(self.messages[idx].clone());
                }
                compressed.push(Message::user(&summary));
                compressed.extend(to_keep);
                self.messages = compressed;
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    message_count = self.messages.len(),
                    "context compression failed, keeping full history"
                );
            }
        }
    }

    /// Ask the LLM to summarise a block of earlier messages into a single
    /// paragraph suitable for resuming the conversation.
    async fn summarize_messages(
        &self,
        provider: &dyn LlmProvider,
        messages: &[&Message],
    ) -> Result<String, AgentError> {
        // Build a compact representation of the messages to summarise.
        let conversation_text: String = messages
            .iter()
            .map(|m| format!("[{}]: {}", m.role_str(), m))
            .collect::<Vec<_>>()
            .join("\n");

        let summary_prompt = format!(
            "Summarize the following conversation into a concise paragraph \
             (no more than 200 words) that preserves all key facts, decisions, \
             and context needed to continue the conversation:\n\n{conversation_text}"
        );

        let request = CompletionRequest::new(
            &self.config.model,
            vec![
                Message::system(
                    "You are a conversation summarizer. Produce a concise, factual summary.",
                ),
                Message::user(&summary_prompt),
            ],
        );

        let response = provider
            .complete(request, RequestOptions::default())
            .await
            .map_err(|e| AgentError::Io(format!("summarization failed: {e}")))?;

        let summary = response.content.unwrap_or_default();
        let full_summary =
            format!("[Summary of previous conversation]\n\n{summary}\n\n[End of summary]");

        Ok(full_summary)
    }

    /// Return the number of messages currently in the conversation.
    pub fn count_messages(&self) -> usize {
        self.messages.len()
    }

    /// Clear all messages from the conversation.
    pub fn clear(&mut self) {
        self.messages.clear();
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use xz_provider::{
        CompletionResponse, FinishReason, ModelInfo, ProviderError, RequestOptions, TokenUsage,
    };

    // ── Mock Provider ──

    /// A minimal mock [`LlmProvider`] that returns pre-configured responses.
    ///
    /// Supports two response modes: text and tool calls.
    #[derive(Debug)]
    struct MockProvider {
        response_mode: MockResponseMode,
    }

    #[derive(Debug, Clone)]
    enum MockResponseMode {
        Text { content: String },
        ToolCalls { calls: Vec<ToolCall> },
    }

    impl MockProvider {
        fn new_text(content: &str) -> Self {
            Self {
                response_mode: MockResponseMode::Text {
                    content: content.to_string(),
                },
            }
        }

        fn new_tool_calls(calls: Vec<ToolCall>) -> Self {
            Self {
                response_mode: MockResponseMode::ToolCalls { calls },
            }
        }

        fn build_response(&self, _model: &str) -> CompletionResponse {
            match &self.response_mode {
                MockResponseMode::Text { content } => CompletionResponse {
                    content: Some(content.clone()),
                    thinking: None,
                    tool_calls: vec![],
                    usage: TokenUsage::new(10, 20),
                    model: _model.to_string(),
                    finish_reason: FinishReason::Stop,
                    latency_ms: 100,
                    cache_info: None,
                },
                MockResponseMode::ToolCalls { calls } => CompletionResponse {
                    content: None,
                    thinking: None,
                    tool_calls: calls.clone(),
                    usage: TokenUsage::new(10, 20),
                    model: _model.to_string(),
                    finish_reason: FinishReason::ToolCall,
                    latency_ms: 100,
                    cache_info: None,
                },
            }
        }
    }

    #[async_trait]
    impl LlmProvider for MockProvider {
        async fn complete(
            &self,
            request: CompletionRequest,
            _options: RequestOptions,
        ) -> Result<CompletionResponse, ProviderError> {
            let model = request.model.unwrap_or_else(|| "mock".to_string());
            Ok(self.build_response(&model))
        }

        async fn complete_stream(
            &self,
            _request: CompletionRequest,
            _options: RequestOptions,
        ) -> Result<
            std::pin::Pin<
                Box<
                    dyn futures::Stream<Item = Result<xz_provider::StreamEvent, ProviderError>>
                        + Send,
                >,
            >,
            ProviderError,
        > {
            Err(ProviderError::Config("streaming not supported in mock".into()))
        }

        fn models(&self) -> &[ModelInfo] {
            &[]
        }

        fn name(&self) -> &str {
            "mock"
        }
    }

    // ── Helpers ──

    fn make_config() -> ConversationConfig {
        ConversationConfig {
            model: "mock-model".to_string(),
            ..Default::default()
        }
    }

    // ── Tests ──

    #[tokio::test]
    async fn test_start_and_get_messages() {
        let mut manager = ConversationManager::new(make_config());
        manager.start("You are helpful.".to_string(), "Hi!".to_string());
        let msgs = manager.get_messages();
        assert_eq!(msgs.len(), 2);
        assert!(matches!(msgs[0], Message::System { .. }));
        assert!(matches!(msgs[1], Message::User { .. }));
    }

    #[tokio::test]
    async fn test_inject_tool_result() {
        let mut manager = ConversationManager::new(make_config());
        manager.start("sys".to_string(), "user".to_string());
        assert_eq!(manager.count_messages(), 2);

        manager.inject_tool_result("call_1", "result text", false);
        assert_eq!(manager.count_messages(), 3);

        let msgs = manager.get_messages();
        assert!(matches!(msgs[2], Message::Tool { .. }));

        manager.inject_tool_result("call_2", "error text", true);
        let msgs = manager.get_messages();
        match &msgs[3] {
            Message::Tool { is_error, .. } => assert!(is_error),
            _ => panic!("expected Tool message"),
        }
    }

    #[tokio::test]
    async fn test_response_text() {
        let provider = MockProvider::new_text("Hello, how can I help?");
        let mut manager = ConversationManager::new(make_config());
        manager.start("sys".to_string(), "user".to_string());

        let response = manager
            .next_response(&provider, None)
            .await
            .expect("next_response should succeed");

        match response {
            ConversationResponse::Text { content } => {
                assert_eq!(content, "Hello, how can I help?");
            }
            _ => panic!("expected Text response"),
        }

        // Assistant message should be appended.
        assert_eq!(manager.count_messages(), 3);
    }

    #[tokio::test]
    async fn test_response_tool_calls() {
        let tool_call = ToolCall {
            id: "call_abc".to_string(),
            function_name: "search".to_string(),
            arguments: serde_json::json!({"query": "rust"}),
        };
        let provider = MockProvider::new_tool_calls(vec![tool_call]);
        let mut manager = ConversationManager::new(make_config());
        manager.start("sys".to_string(), "user".to_string());

        let response = manager
            .next_response(&provider, None)
            .await
            .expect("next_response should succeed");

        match response {
            ConversationResponse::ToolCalls { calls } => {
                assert_eq!(calls.len(), 1);
                assert_eq!(calls[0].function_name, "search");
            }
            _ => panic!("expected ToolCalls response"),
        }

        // Assistant message (with tool_calls) should be appended.
        let msgs = manager.get_messages();
        assert_eq!(msgs.len(), 3);
        assert!(matches!(msgs[2], Message::Assistant { .. }));
    }

    #[tokio::test]
    async fn test_maybe_compress_triggers_on_threshold() {
        let mut config = make_config();
        config.max_context_messages = 6;
        config.compression_enabled = true;
        let mut manager = ConversationManager::new(config);

        // Build a conversation that exceeds the threshold.
        manager.start("System prompt".to_string(), "Q1".to_string());
        // Messages: System, User(Q1) — 2 total

        // Add enough messages to exceed threshold=6.
        // After start we have 2 messages. Add 5 more assistant messages.
        for i in 2..=6 {
            manager.append_message(Message::assistant(&format!("A{i}")));
        }
        assert_eq!(manager.count_messages(), 7);
        assert!(manager.count_messages() > manager.config.max_context_messages);

        // Compression summarises the old messages.  The mock provider
        // returns a fixed text, so the summary message will be inserted.
        let provider = MockProvider::new_text("This is a summary.");
        manager.maybe_compress(&provider).await;

        // After compression: System, User(summary), + the 3 most recent messages
        // (keep_count = 6/2 = 3)
        assert!(
            manager.count_messages() <= manager.config.max_context_messages,
            "expected {} messages after compression, got {}",
            manager.config.max_context_messages,
            manager.count_messages()
        );
    }

    #[tokio::test]
    async fn test_maybe_compress_skips_when_disabled() {
        let mut config = make_config();
        config.max_context_messages = 2; // very low threshold
        config.compression_enabled = false;
        let mut manager = ConversationManager::new(config);

        manager.start("sys".to_string(), "q1".to_string());
        manager.append_message(Message::assistant("a1"));
        manager.append_message(Message::assistant("a2"));
        // 4 messages total, threshold is 2.
        assert_eq!(manager.count_messages(), 4);

        let provider = MockProvider::new_text("summary");
        manager.maybe_compress(&provider).await;

        // Should not compress because compression is disabled.
        assert_eq!(manager.count_messages(), 4);
    }

    #[tokio::test]
    async fn test_clear() {
        let mut manager = ConversationManager::new(make_config());
        manager.start("sys".to_string(), "user".to_string());
        assert_eq!(manager.count_messages(), 2);
        manager.clear();
        assert_eq!(manager.count_messages(), 0);
        assert!(manager.get_messages().is_empty());
    }
}
