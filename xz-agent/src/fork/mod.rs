//! Fork management for spawning parallel sub-agents.
//!
//! This module provides [`ForkManager`], which orchestrates the creation and
//! execution of independent sub-agents (forks). Each fork has its own
//! conversation context and restricted tool set. Sub-agents cannot
//! recursively fork.
//!
//! # Overview
//!
//! 1. Call [`ForkManager::spawn_fork`] to create a new fork with its own
//!    tools and context. This returns a [`uuid::Uuid`] immediately.
//! 2. Call [`ForkManager::run_fork`] to execute a single fork, or
//!    [`ForkManager::run_all_forks`] to execute all pending forks concurrently.
//! 3. Inspect results via [`ForkManager::get_handle`] or collect completed
//!    results with [`ForkManager::collect_results`].
//!
//! # Concurrency
//!
//! [`ForkManager::run_all_forks`] uses [`tokio::task::spawn`] and
//! [`futures::future::join_all`] to execute forks concurrently. Each fork
//! is isolated — no mutable state is shared between forks.
//!
//! # Sub-agent Constraints
//!
//! Each fork receives only its provided tools. The [`ForkManager`] is
//! intentionally not exposed to forked sub-agents, preventing recursive
//! forking.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AgentError;
use crate::tool::{AgentTool, ToolContext, ToolOutput, ToolRegistry};
use crate::trajectory::{AgentTrajectory, TrajectoryAction};

use xz_provider::{
    CompletionRequest, FinishReason, LlmProvider, Message, MessageContent, RequestOptions, ToolCall,
};

// ── ForkConfig ──

/// Configuration controlling fork execution behaviour.
///
/// Limits concurrency, tool calls, and overall execution time to prevent
/// runaway sub-agents from consuming excessive resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkConfig {
    /// Maximum number of forks that may execute concurrently.
    pub max_concurrent_forks: usize,

    /// Maximum tool calls a single fork may make before termination.
    pub max_tool_calls_per_fork: u32,

    /// Maximum wall-clock seconds a fork may execute before timing out.
    pub timeout_secs: u64,
}

impl Default for ForkConfig {
    fn default() -> Self {
        Self {
            max_concurrent_forks: 3,
            max_tool_calls_per_fork: 10,
            timeout_secs: 300,
        }
    }
}

// ── ForkStatus ──

/// Lifecycle state of a fork.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ForkStatus {
    /// Created but not yet started.
    Pending,

    /// Currently executing.
    Running,

    /// Completed successfully.
    Completed,

    /// Failed with an error message.
    Failed {
        /// Human-readable error description.
        error: String,
    },

    /// Exceeded the configured timeout.
    TimedOut,
}

// ── ForkResult ──

/// Result produced by a completed fork execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkResult {
    /// The fork that produced this result.
    pub fork_id: Uuid,

    /// All tool outputs generated during execution.
    pub tool_outputs: Vec<ToolOutput>,

    /// Complete execution trajectory.
    pub trajectory: AgentTrajectory,

    /// Total number of tool calls made.
    pub total_tool_calls: u32,

    /// Wall-clock duration in milliseconds.
    pub duration_ms: u64,
}

// ── ForkHandle ──

/// Handle representing a spawned fork and its lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkHandle {
    /// Unique fork identifier.
    pub fork_id: Uuid,

    /// Human-readable task description.
    pub task_description: String,

    /// Current lifecycle status.
    pub status: ForkStatus,

    /// When the fork was created.
    pub created_at: DateTime<Utc>,

    /// When execution completed, if it has.
    pub completed_at: Option<DateTime<Utc>>,

    /// The execution result, if completed.
    pub result: Option<ForkResult>,
}

// ── Internal state ──

/// Internal execution state stored alongside each handle.
/// Consumed during execution; not exposed in the public API.
struct ForkInternal {
    tools: Vec<Box<dyn AgentTool>>,
    tool_context: ToolContext,
}

// ── ForkManager ──

/// Manages spawning, execution, and result collection for sub-agent forks.
pub struct ForkManager {
    config: ForkConfig,
    handles: Vec<ForkHandle>,
    internal: HashMap<Uuid, ForkInternal>,
}

impl ForkManager {
    /// Creates a new [`ForkManager`] with the given configuration.
    pub fn new(config: ForkConfig) -> Self {
        Self {
            config,
            handles: Vec::new(),
            internal: HashMap::new(),
        }
    }

    /// Spawns a new fork in [`ForkStatus::Pending`] state.
    ///
    /// Call [`run_fork`](Self::run_fork) or [`run_all_forks`](Self::run_all_forks)
    /// to begin execution.
    pub fn spawn_fork(
        &mut self,
        task_description: &str,
        tools: Vec<Box<dyn AgentTool>>,
        tool_context: ToolContext,
    ) -> Uuid {
        let fork_id = Uuid::new_v4();
        let now = Utc::now();

        let handle = ForkHandle {
            fork_id,
            task_description: task_description.to_string(),
            status: ForkStatus::Pending,
            created_at: now,
            completed_at: None,
            result: None,
        };

        self.handles.push(handle);
        self.internal.insert(fork_id, ForkInternal {
            tools,
            tool_context,
        });

        fork_id
    }

    /// Executes a specific fork by ID.
    ///
    /// The `_provider` and `_tool_registry` parameters are accepted for API
    /// compatibility but not used — each fork uses the provider from its
    /// stored [`ToolContext`] and its own restricted tool set.
    pub async fn run_fork(
        &mut self,
        fork_id: Uuid,
        _provider: &dyn LlmProvider,
        _tool_registry: &ToolRegistry,
    ) -> Result<ForkResult, AgentError> {
        let idx = self
            .handles
            .iter()
            .position(|h| h.fork_id == fork_id)
            .ok_or_else(|| AgentError::NotFound(format!("fork '{fork_id}' not found")))?;

        let internal = self
            .internal
            .remove(&fork_id)
            .ok_or_else(|| AgentError::Io(format!("fork '{fork_id}' missing internal state")))?;

        let task_description = self.handles[idx].task_description.clone();
        let novel_id = internal.tool_context.novel_id.clone();
        let chapter_number = internal.tool_context.chapter_number;
        let provider = internal.tool_context.provider.clone();

        self.handles[idx].status = ForkStatus::Running;

        let timeout_dur = std::time::Duration::from_secs(self.config.timeout_secs);
        let run_result = tokio::time::timeout(
            timeout_dur,
            execute_fork(
                fork_id,
                task_description,
                novel_id,
                chapter_number,
                internal.tools,
                provider,
                internal.tool_context,
                self.config.clone(),
            ),
        )
        .await;

        match run_result {
            Ok(Ok(result)) => {
                self.handles[idx].status = ForkStatus::Completed;
                self.handles[idx].completed_at = Some(Utc::now());
                self.handles[idx].result = Some(result.clone());
                Ok(result)
            }
            Ok(Err(e)) => {
                self.handles[idx].status = ForkStatus::Failed {
                    error: e.to_string(),
                };
                self.handles[idx].completed_at = Some(Utc::now());
                Err(e)
            }
            Err(_elapsed) => {
                self.handles[idx].status = ForkStatus::TimedOut;
                self.handles[idx].completed_at = Some(Utc::now());
                Err(AgentError::Timeout(self.config.timeout_secs))
            }
        }
    }

    /// Runs all pending forks concurrently via [`tokio::task::spawn`].
    pub async fn run_all_forks(
        &mut self,
        _provider: &dyn LlmProvider,
        _tool_registry: &ToolRegistry,
    ) -> Vec<Result<ForkResult, AgentError>> {
        struct PendingFork {
            idx: usize,
            fork_id: Uuid,
            task_description: String,
            novel_id: String,
            chapter_number: u32,
            tools: Vec<Box<dyn AgentTool>>,
            provider: Arc<dyn LlmProvider>,
            tool_context: ToolContext,
        }

        let mut pending_forks: Vec<PendingFork> = Vec::new();

        for (idx, handle) in self.handles.iter_mut().enumerate() {
            if handle.status != ForkStatus::Pending {
                continue;
            }

            let internal = match self.internal.remove(&handle.fork_id) {
                Some(i) => i,
                None => continue,
            };

            handle.status = ForkStatus::Running;

            pending_forks.push(PendingFork {
                idx,
                fork_id: handle.fork_id,
                task_description: handle.task_description.clone(),
                novel_id: internal.tool_context.novel_id.clone(),
                chapter_number: internal.tool_context.chapter_number,
                tools: internal.tools,
                provider: internal.tool_context.provider.clone(),
                tool_context: internal.tool_context,
            });
        }

        if pending_forks.is_empty() {
            return Vec::new();
        }

        let config = self.config.clone();
        let mut tasks: Vec<(usize, tokio::task::JoinHandle<Result<ForkResult, AgentError>>)> =
            Vec::with_capacity(pending_forks.len());

        for pf in pending_forks {
            let cfg = config.clone();
            let task = tokio::spawn(execute_fork(
                pf.fork_id,
                pf.task_description,
                pf.novel_id,
                pf.chapter_number,
                pf.tools,
                pf.provider,
                pf.tool_context,
                cfg,
            ));
            tasks.push((pf.idx, task));
        }

        let mut results = Vec::with_capacity(tasks.len());

        for (idx, task) in tasks {
            match task.await {
                Ok(Ok(result)) => {
                    self.handles[idx].status = ForkStatus::Completed;
                    self.handles[idx].completed_at = Some(Utc::now());
                    self.handles[idx].result = Some(result.clone());
                    results.push(Ok(result));
                }
                Ok(Err(e)) => {
                    self.handles[idx].status = ForkStatus::Failed {
                        error: e.to_string(),
                    };
                    self.handles[idx].completed_at = Some(Utc::now());
                    results.push(Err(e));
                }
                Err(join_err) => {
                    let err = AgentError::Io(format!(
                        "fork task panicked or was cancelled: {join_err}"
                    ));
                    self.handles[idx].status = ForkStatus::Failed {
                        error: err.to_string(),
                    };
                    self.handles[idx].completed_at = Some(Utc::now());
                    results.push(Err(err));
                }
            }
        }

        results
    }

    /// Looks up a fork handle by ID.
    pub fn get_handle(&self, fork_id: Uuid) -> Option<&ForkHandle> {
        self.handles.iter().find(|h| h.fork_id == fork_id)
    }

    /// Returns all fork handles.
    pub fn get_handles(&self) -> &[ForkHandle] {
        &self.handles
    }

    /// Returns the count of forks in [`ForkStatus::Pending`] state.
    pub fn pending_count(&self) -> usize {
        self.handles
            .iter()
            .filter(|h| h.status == ForkStatus::Pending)
            .count()
    }

    /// Returns the count of forks in [`ForkStatus::Completed`] state.
    pub fn completed_count(&self) -> usize {
        self.handles
            .iter()
            .filter(|h| h.status == ForkStatus::Completed)
            .count()
    }

    /// Cancels a pending or running fork, marking it as failed.
    pub fn cancel_fork(&mut self, fork_id: Uuid) {
        if let Some(handle) = self.handles.iter_mut().find(|h| h.fork_id == fork_id) {
            if handle.status == ForkStatus::Pending || handle.status == ForkStatus::Running {
                handle.status = ForkStatus::Failed {
                    error: "cancelled".to_string(),
                };
                handle.completed_at = Some(Utc::now());
            }
        }
    }

    /// Collects completed fork results and removes them from tracking.
    pub fn collect_results(&mut self) -> Vec<ForkResult> {
        let mut results = Vec::new();
        let mut to_remove: Vec<Uuid> = Vec::new();

        for handle in &self.handles {
            if handle.status == ForkStatus::Completed {
                if let Some(ref result) = handle.result {
                    results.push(result.clone());
                    to_remove.push(handle.fork_id);
                }
            }
        }

        for fork_id in &to_remove {
            self.handles.retain(|h| h.fork_id != *fork_id);
            self.internal.remove(fork_id);
        }

        results
    }
}

// ── Core fork execution loop ──

/// Runs the LLM interaction loop for a single fork.
///
/// Takes ownership of all data so it can be passed to [`tokio::spawn`]
/// for concurrent execution in [`ForkManager::run_all_forks`].
async fn execute_fork(
    fork_id: Uuid,
    task_description: String,
    novel_id: String,
    chapter_number: u32,
    mut tools: Vec<Box<dyn AgentTool>>,
    provider: Arc<dyn LlmProvider>,
    tool_context: ToolContext,
    config: ForkConfig,
) -> Result<ForkResult, AgentError> {
    let start = Instant::now();

    // Fork-local tool registry
    let mut local_registry = ToolRegistry::new();
    for tool in tools.drain(..) {
        local_registry.register(tool)?;
    }
    let tool_defs = local_registry.list_definitions();

    let mut trajectory = AgentTrajectory::new(
        fork_id.to_string(),
        novel_id,
        chapter_number,
    );

    let mut messages: Vec<Message> = vec![Message::system(&task_description)];
    let mut tool_outputs: Vec<ToolOutput> = Vec::new();
    let mut total_tool_calls: u32 = 0;

    loop {
        if total_tool_calls >= config.max_tool_calls_per_fork {
            trajectory.record_step(
                TrajectoryAction::Thought {
                    content: "[max tool calls reached]".into(),
                },
                None,
            );
            break;
        }

        let tools_for_request = if tool_defs.is_empty() {
            None
        } else {
            Some(tool_defs.clone())
        };

        let request = CompletionRequest {
            model: None,
            messages: messages.clone(),
            tools: tools_for_request,
            tool_choice: None,
            response_format: None,
            temperature: None,
            max_tokens: None,
            max_completion_tokens: None,
            top_p: None,
            top_k: None,
            stop: None,
            frequency_penalty: None,
            presence_penalty: None,
            seed: None,
            reasoning_effort: None,
            logprobs: None,
            logit_bias: None,
            stream_include_usage: None,
            request_id: Uuid::new_v4().to_string(),
        };

        let options = RequestOptions {
            timeout: Some(std::time::Duration::from_secs(120)),
            ..Default::default()
        };

        let resp = provider
            .complete(request, options)
            .await
            .map_err(|e| AgentError::Io(format!("LLM provider error: {e}")))?;

        let mut assistant_content: Option<MessageContent> = None;
        let mut assistant_tool_calls: Option<Vec<ToolCall>> = None;

        if let Some(ref content) = resp.content {
            if !content.is_empty() {
                trajectory.record_step(
                    TrajectoryAction::Thought {
                        content: content.clone(),
                    },
                    Some(content.clone()),
                );
                assistant_content = Some(MessageContent::from(content.as_str()));
            }
        }

        if !resp.tool_calls.is_empty() {
            assistant_tool_calls = Some(resp.tool_calls.clone());

            for tool_call in &resp.tool_calls {
                total_tool_calls += 1;

                let call_start = Instant::now();
                let output = local_registry
                    .execute(
                        &tool_call.function_name,
                        &tool_context,
                        tool_call.arguments.clone(),
                    )
                    .await;

                let duration_ms = call_start.elapsed().as_millis() as u64;
                let result_str = match &output {
                    Ok(o) => o.content.clone(),
                    Err(e) => format!("error: {e}"),
                };

                trajectory.record_step(
                    TrajectoryAction::ToolCall {
                        tool_name: tool_call.function_name.clone(),
                        arguments: tool_call.arguments.clone(),
                        result: Some(result_str),
                        duration_ms,
                    },
                    None,
                );

                match output {
                    Ok(out) => {
                        tool_outputs.push(out.clone());
                        messages.push(Message::tool_result(&tool_call.id, &out.content));
                    }
                    Err(e) => {
                        let err_content = format!("{e}");
                        tool_outputs.push(ToolOutput {
                            content: err_content.clone(),
                            structured: None,
                            is_error: true,
                            tool_call_id: Some(tool_call.id.clone()),
                        });
                        messages.push(Message::tool_error(&tool_call.id, &err_content));
                    }
                }
            }
        }

        messages.push(Message::Assistant {
            content: assistant_content.unwrap_or(MessageContent::None),
            tool_calls: assistant_tool_calls,
            cache_control: None,
        });

        match resp.finish_reason {
            FinishReason::Stop | FinishReason::MaxTokens | FinishReason::ContentFilter => {
                if resp.tool_calls.is_empty() {
                    break;
                }
            }
            FinishReason::ToolCall => {}
        }
    }

    trajectory.mark_completed();
    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(ForkResult {
        fork_id,
        tool_outputs,
        trajectory,
        total_tool_calls,
        duration_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use xz_provider::CompletionResponse;

    // ── Test helpers ──

    struct MockTool {
        name: String,
        description: String,
        schema: serde_json::Value,
    }

    #[async_trait::async_trait]
    impl AgentTool for MockTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            &self.description
        }

        fn parameter_schema(&self) -> serde_json::Value {
            self.schema.clone()
        }

        async fn execute(
            &self,
            _context: &ToolContext,
            _args: serde_json::Value,
        ) -> Result<ToolOutput, AgentError> {
            Ok(ToolOutput {
                content: "mock result".to_string(),
                structured: None,
                is_error: false,
                tool_call_id: None,
            })
        }
    }

    fn mock_tool(name: &str) -> Box<dyn AgentTool> {
        Box::new(MockTool {
            name: name.to_string(),
            description: format!("Mock tool: {name}"),
            schema: json!({"type": "object", "properties": {}}),
        })
    }

    #[derive(Debug)]
    struct DummyProvider;

    #[async_trait::async_trait]
    impl LlmProvider for DummyProvider {
        async fn complete(
            &self,
            _request: CompletionRequest,
            _options: RequestOptions,
        ) -> Result<CompletionResponse, xz_provider::ProviderError> {
            unreachable!("DummyProvider::complete should not be called in fork unit tests")
        }

        async fn complete_stream(
            &self,
            _request: CompletionRequest,
            _options: RequestOptions,
        ) -> Result<
            std::pin::Pin<
                Box<
                    dyn futures::Stream<Item = Result<xz_provider::StreamEvent, xz_provider::ProviderError>>
                        + Send,
                >,
            >,
            xz_provider::ProviderError,
        > {
            unreachable!("DummyProvider::complete_stream should not be called")
        }

        fn models(&self) -> &[xz_provider::ModelInfo] {
            &[]
        }

        fn name(&self) -> &str {
            "dummy"
        }
    }

    #[derive(Debug)]
    struct DummyMemory;

    #[async_trait::async_trait]
    impl xz_memory::MemorySystem for DummyMemory {
        async fn append_message(
            &self,
            _session_id: &str,
            _msg: xz_memory::Message,
        ) -> Result<(), xz_memory::MemoryError> {
            Ok(())
        }

        async fn get_recent_messages(
            &self,
            _session_id: &str,
            _n: usize,
        ) -> Result<Vec<xz_memory::Message>, xz_memory::MemoryError> {
            Ok(vec![])
        }

        async fn get_session_messages(
            &self,
            _session_id: &str,
            _page: xz_memory::PageRequest,
        ) -> Result<xz_memory::MessagePage, xz_memory::MemoryError> {
            Ok(xz_memory::MessagePage {
                items: vec![],
                total: 0,
                has_more: false,
            })
        }

        async fn clear_short_term(&self, _session_id: &str) -> Result<(), xz_memory::MemoryError> {
            Ok(())
        }

        async fn evict_oldest_messages(
            &self,
            _session_id: &str,
            _keep_count: usize,
        ) -> Result<usize, xz_memory::MemoryError> {
            Ok(0)
        }

        async fn update_summary(
            &self,
            _session_id: &str,
            _summary: xz_memory::SessionSummary,
        ) -> Result<(), xz_memory::MemoryError> {
            Ok(())
        }

        async fn get_summary_history(
            &self,
            _user_id: &str,
            _limit: usize,
        ) -> Result<Vec<xz_memory::SessionSummary>, xz_memory::MemoryError> {
            Ok(vec![])
        }

        async fn remember_fact(
            &self,
            _fact: xz_memory::Fact,
        ) -> Result<xz_memory::UpsertResult, xz_memory::MemoryError> {
            Ok(xz_memory::UpsertResult::Created)
        }

        async fn recall_facts(
            &self,
            _user_id: &str,
            _query: &str,
            _options: &xz_memory::FactRecallOptions,
        ) -> Result<xz_memory::FactPage, xz_memory::MemoryError> {
            Ok(xz_memory::FactPage {
                items: vec![],
                total: 0,
                has_more: false,
            })
        }

        async fn get_user_preferences(
            &self,
            _user_id: &str,
        ) -> Result<Vec<xz_memory::Fact>, xz_memory::MemoryError> {
            Ok(vec![])
        }

        async fn delete_fact(&self, _id: &str) -> Result<(), xz_memory::MemoryError> {
            Ok(())
        }

        async fn compact_facts(
            &self,
            _user_id: &str,
            _strategy: xz_memory::CompactionStrategy,
        ) -> Result<xz_memory::CompactionResult, xz_memory::MemoryError> {
            Ok(xz_memory::CompactionResult {
                facts_merged: 0,
                facts_removed: 0,
                facts_kept: 0,
            })
        }

        async fn stats(&self, _user_id: &str) -> Result<xz_memory::MemoryStats, xz_memory::MemoryError> {
            Ok(xz_memory::MemoryStats {
                total_sessions: 0,
                total_messages: 0,
                total_facts: 0,
                total_vectors: 0,
                total_tokens_approx: 0,
                db_size_bytes: 0,
            })
        }

        async fn export(&self, _user_id: &str) -> Result<xz_memory::MemoryExport, xz_memory::MemoryError> {
            Ok(xz_memory::MemoryExport {
                version: "1.0".into(),
                user_id: _user_id.into(),
                exported_at: 0,
                sessions: vec![],
                facts: vec![],
                vectors: vec![],
            })
        }

        async fn import(
            &self,
            _data: xz_memory::MemoryExport,
        ) -> Result<xz_memory::ImportResult, xz_memory::MemoryError> {
            Ok(xz_memory::ImportResult {
                sessions_imported: 0,
                facts_imported: 0,
                vectors_imported: 0,
            })
        }

        async fn get_or_create_summary(
            &self,
            _session_id: &str,
            _provider: &dyn xz_provider::LlmProvider,
        ) -> Result<xz_memory::SessionSummary, xz_memory::MemoryError> {
            Ok(xz_memory::SessionSummary {
                session_id: _session_id.into(),
                user_id: "test-user".into(),
                summary: "summary".into(),
                key_points: vec![],
                token_count: 0,
                message_count: 0,
                created_at: 0,
                updated_at: 0,
            })
        }
    }

    fn dummy_tool_context() -> ToolContext {
        ToolContext {
            novel_id: "test-novel".to_string(),
            chapter_number: 1,
            provider: Arc::new(DummyProvider),
            memory: Arc::new(DummyMemory),
            knowledge_graph: None,
        }
    }

    // ── Tests ──

    #[test]
    fn test_fork_manager_new() {
        let manager = ForkManager::new(ForkConfig::default());
        assert_eq!(manager.pending_count(), 0);
        assert_eq!(manager.completed_count(), 0);
        assert!(manager.get_handles().is_empty());
    }

    #[test]
    fn test_spawn_fork() {
        let mut manager = ForkManager::new(ForkConfig::default());
        let fork_id = manager.spawn_fork("research quantum computing", vec![], dummy_tool_context());

        let handle = manager.get_handle(fork_id).expect("handle should exist");
        assert_eq!(handle.fork_id, fork_id);
        assert_eq!(handle.task_description, "research quantum computing");
        assert_eq!(handle.status, ForkStatus::Pending);
        assert!(handle.created_at <= Utc::now());
        assert!(handle.completed_at.is_none());
        assert!(handle.result.is_none());
    }

    #[test]
    fn test_spawn_fork_with_tools() {
        let mut manager = ForkManager::new(ForkConfig::default());
        let tools: Vec<Box<dyn AgentTool>> = vec![mock_tool("search"), mock_tool("read")];
        let fork_id = manager.spawn_fork("task with tools", tools, dummy_tool_context());

        let handle = manager.get_handle(fork_id).expect("handle should exist");
        assert_eq!(handle.status, ForkStatus::Pending);
        assert_eq!(handle.task_description, "task with tools");
    }

    #[test]
    fn test_pending_and_completed_counts() {
        let mut manager = ForkManager::new(ForkConfig::default());
        manager.spawn_fork("task 1", vec![], dummy_tool_context());
        manager.spawn_fork("task 2", vec![], dummy_tool_context());
        manager.spawn_fork("task 3", vec![], dummy_tool_context());

        assert_eq!(manager.pending_count(), 3);
        assert_eq!(manager.completed_count(), 0);
    }

    #[test]
    fn test_cancel_fork() {
        let mut manager = ForkManager::new(ForkConfig::default());
        let fork_id = manager.spawn_fork("cancellable", vec![], dummy_tool_context());

        manager.cancel_fork(fork_id);

        let handle = manager.get_handle(fork_id).expect("handle should exist");
        match &handle.status {
            ForkStatus::Failed { error } => assert_eq!(error, "cancelled"),
            other => panic!("expected Failed status, got {other:?}"),
        }
        assert!(handle.completed_at.is_some());
    }

    #[test]
    fn test_cancel_fork_idempotent() {
        let mut manager = ForkManager::new(ForkConfig::default());
        let fork_id = manager.spawn_fork("test", vec![], dummy_tool_context());

        manager.cancel_fork(fork_id);
        manager.cancel_fork(fork_id);

        let handle = manager.get_handle(fork_id).expect("handle should exist");
        assert!(matches!(handle.status, ForkStatus::Failed { .. }));
    }

    #[test]
    fn test_collect_results_empty() {
        let mut manager = ForkManager::new(ForkConfig::default());
        let results = manager.collect_results();
        assert!(results.is_empty());
    }

    #[test]
    fn test_collect_results_removes_completed() {
        let mut manager = ForkManager::new(ForkConfig::default());
        let fork_id = manager.spawn_fork("will complete", vec![], dummy_tool_context());

        // Simulate a completed fork
        if let Some(handle) = manager.handles.iter_mut().find(|h| h.fork_id == fork_id) {
            handle.status = ForkStatus::Completed;
            handle.completed_at = Some(Utc::now());
            handle.result = Some(ForkResult {
                fork_id,
                tool_outputs: vec![],
                trajectory: AgentTrajectory::new("test", "test-novel", 1),
                total_tool_calls: 0,
                duration_ms: 0,
            });
        }

        assert_eq!(manager.completed_count(), 1);
        let results = manager.collect_results();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].fork_id, fork_id);

        assert!(manager.get_handle(fork_id).is_none());
        assert_eq!(manager.completed_count(), 0);
    }

    #[test]
    fn test_fork_config_default() {
        let config = ForkConfig::default();
        assert_eq!(config.max_concurrent_forks, 3);
        assert_eq!(config.max_tool_calls_per_fork, 10);
        assert_eq!(config.timeout_secs, 300);
    }

    #[test]
    fn test_get_handles_returns_all() {
        let mut manager = ForkManager::new(ForkConfig::default());
        manager.spawn_fork("a", vec![], dummy_tool_context());
        manager.spawn_fork("b", vec![], dummy_tool_context());

        let handles = manager.get_handles();
        assert_eq!(handles.len(), 2);
    }

    #[test]
    fn test_cancel_nonexistent_fork_does_not_panic() {
        let mut manager = ForkManager::new(ForkConfig::default());
        let nonexistent = Uuid::new_v4();
        manager.cancel_fork(nonexistent);
    }

    #[test]
    fn test_fork_handle_serialization() {
        let fork_id = Uuid::new_v4();
        let handle = ForkHandle {
            fork_id,
            task_description: "serialize test".into(),
            status: ForkStatus::Pending,
            created_at: Utc::now(),
            completed_at: None,
            result: None,
        };

        let json = serde_json::to_string(&handle).expect("serialization should succeed");
        let deserialized: ForkHandle =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(deserialized.fork_id, fork_id);
        assert_eq!(deserialized.task_description, "serialize test");
        assert_eq!(deserialized.status, ForkStatus::Pending);
    }

    #[test]
    fn test_fork_status_serialization() {
        let statuses = vec![
            ForkStatus::Pending,
            ForkStatus::Running,
            ForkStatus::Completed,
            ForkStatus::Failed {
                error: "oops".into(),
            },
            ForkStatus::TimedOut,
        ];

        for status in &statuses {
            let json = serde_json::to_string(status).expect("serialize");
            let deserialized: ForkStatus =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(deserialized, *status);
        }
    }

    #[test]
    fn test_fork_result_serialization() {
        let result = ForkResult {
            fork_id: Uuid::new_v4(),
            tool_outputs: vec![],
            trajectory: AgentTrajectory::new("sess", "novel", 1),
            total_tool_calls: 5,
            duration_ms: 1234,
        };

        let json = serde_json::to_string(&result).expect("serialize");
        let deserialized: ForkResult =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.total_tool_calls, 5);
        assert_eq!(deserialized.duration_ms, 1234);
    }
}
