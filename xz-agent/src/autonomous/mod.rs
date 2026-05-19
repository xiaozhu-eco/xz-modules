//! Autonomous agent execution loop.
//!
//! [`AutonomousLoop`] integrates [`ConversationManager`](crate::conversation::ConversationManager),
//! [`ToolRegistry`](crate::tool::ToolRegistry), [`SafetyGuard`](crate::safety::SafetyGuard),
//! [`ForkManager`](crate::fork::ForkManager), and [`AgentTrajectory`](crate::trajectory::AgentTrajectory)
//! into a single coordinated execution flow.
//!
//! # Architecture
//!
//! The loop follows a Think → Act → Verify → Complete pattern:
//!
//! 1. **Start** — Initialise conversation with system prompt and task.
//! 2. **Think** — LLM produces text or requests tools.
//! 3. **Act** — Execute tool calls via the registry.
//! 4. **Verify** — Safety checks after each tool call.
//! 5. **Complete** — LLM signals `CHAPTER_COMPLETE`.
//! 6. **Finalize** — Run full safety check, produce verdict.
//!
//! # Example
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use xz_agent::autonomous::{AutonomousLoop, AutonomousConfig};
//! use xz_agent::tool::ToolRegistry;
//!
//! let config = AutonomousConfig {
//!     novel_id: "novel-1".into(),
//!     chapter_number: 1,
//!     ..AutonomousConfig::default()
//! };
//! let registry = Arc::new(ToolRegistry::new());
//! let mut loop_ = AutonomousLoop::new(config, registry);
//! ```

use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use uuid::Uuid;

use crate::conversation::{ConversationConfig, ConversationManager, ConversationResponse};
use crate::fork::{ForkConfig, ForkManager};
use crate::safety::{
    FinalVerdict, SafetyCheckContext, SafetyCheckType, SafetyGuard, SafetyReport, SafetyRule,
    SafetySeverity,
};
use crate::tool::{AgentTool, ToolContext, ToolRegistry};
use crate::trajectory::{AgentTrajectory, TrajectoryAction};

use xz_knowledge_graph::KnowledgeGraph;
use xz_memory::MemorySystem;
use xz_provider::{LlmProvider, Message, MessageContent, ToolDefinition};

const CHAPTER_COMPLETE_MARKER: &str = "CHAPTER_COMPLETE";

// ── AutonomousConfig ──

/// Configuration for [`AutonomousLoop`].
///
/// Controls model selection, execution limits, and fork behaviour.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousConfig {
    /// Model name passed to the LLM provider.
    pub model: String,
    /// Maximum number of tool calls allowed per execution.
    pub max_tool_calls: u32,
    /// Maximum number of revision rounds (LLM retries after warnings).
    pub max_revision_rounds: u32,
    /// Sampling temperature for LLM completions.
    pub temperature: Option<f32>,
    /// Maximum completion tokens.
    pub max_tokens: Option<usize>,
    /// Identifier for the novel being written.
    pub novel_id: String,
    /// The chapter number the agent is working on.
    pub chapter_number: u32,
    /// Whether sub-agent forks are enabled.
    pub fork_enabled: bool,
    /// Maximum number of concurrent forks when forking is enabled.
    pub max_concurrent_forks: usize,
}

impl Default for AutonomousConfig {
    fn default() -> Self {
        Self {
            model: "gpt-4o".to_string(),
            max_tool_calls: 50,
            max_revision_rounds: 5,
            temperature: None,
            max_tokens: None,
            novel_id: "default-novel".to_string(),
            chapter_number: 1,
            fork_enabled: false,
            max_concurrent_forks: 3,
        }
    }
}

// ── AutonomousResult ──

/// The result of executing an [`AutonomousLoop`] run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousResult {
    /// The generated chapter content, if the agent produced any.
    pub chapter_content: Option<String>,
    /// Complete execution trajectory with every step recorded.
    pub trajectory: AgentTrajectory,
    /// Full safety report after the final check.
    pub safety_report: SafetyReport,
    /// Total number of tool calls executed.
    pub total_tool_calls: u32,
    /// Total number of steps (thoughts + tool calls + interventions).
    pub total_steps: u32,
    /// Wall-clock duration of the entire run in milliseconds.
    pub duration_ms: u64,
    /// Final verdict on the execution.
    pub final_verdict: FinalVerdict,
}

// ── AutonomousLoop ──

/// The main agentic loop that orchestrates LLM interaction, tool execution,
/// safety checks, and trajectory recording.
pub struct AutonomousLoop {
    config: AutonomousConfig,
    conversation: ConversationManager,
    tool_registry: Arc<ToolRegistry>,
    safety_guard: SafetyGuard,
    trajectory: AgentTrajectory,
    fork_manager: Option<ForkManager>,
}

impl AutonomousLoop {
    /// Creates a new [`AutonomousLoop`] with the given configuration and
    /// tool registry.
    pub fn new(config: AutonomousConfig, tool_registry: Arc<ToolRegistry>) -> Self {
        let session_id = Uuid::new_v4().to_string();
        let trajectory = AgentTrajectory::new(
            session_id,
            config.novel_id.clone(),
            config.chapter_number,
        );

        let conversation = ConversationManager::new(ConversationConfig {
            model: config.model.clone(),
            max_tokens: config.max_tokens,
            temperature: config.temperature,
            max_context_messages: 50,
            compression_enabled: true,
        });

        let mut safety_guard = SafetyGuard::default();
        safety_guard.remove_rule(SafetyCheckType::MaxToolCalls);
        safety_guard.add_rule(SafetyRule::new(
            SafetyCheckType::MaxToolCalls,
            SafetySeverity::Blocking,
            serde_json::json!(config.max_tool_calls),
            "Maximum tool calls per execution",
        ));

        let fork_manager = if config.fork_enabled {
            Some(ForkManager::new(ForkConfig {
                max_concurrent_forks: config.max_concurrent_forks,
                ..ForkConfig::default()
            }))
        } else {
            None
        };

        Self {
            config,
            conversation,
            tool_registry,
            safety_guard,
            trajectory,
            fork_manager,
        }
    }

    /// Runs the main autonomous loop for the given task.
    ///
    /// 1. Builds a dynamic system prompt.
    /// 2. Starts the conversation.
    /// 3. Loops, calling the LLM and executing tool calls until the agent
    ///    signals `CHAPTER_COMPLETE` or a safety limit is breached.
    /// 4. Runs a final safety check and produces an [`AutonomousResult`].
    pub async fn run(
        &mut self,
        task_description: &str,
        provider: Arc<dyn LlmProvider>,
        memory: Arc<dyn MemorySystem>,
        knowledge_graph: Option<Arc<dyn KnowledgeGraph>>,
    ) -> AutonomousResult {
        let start = Instant::now();
        let mut total_tool_calls: u32 = 0;
        let mut revision_rounds: u32 = 0;

        let tool_context = ToolContext {
            novel_id: self.config.novel_id.clone(),
            chapter_number: self.config.chapter_number,
            provider: Arc::clone(&provider),
            memory: Arc::clone(&memory),
            knowledge_graph: knowledge_graph.clone(),
        };

        let system_prompt = self.build_system_prompt(task_description);
        self.conversation
            .start(system_prompt, format!("Begin work on: {task_description}"));

        let tool_definitions = self.tool_registry.list_definitions();
        let mut loop_iterations: u32 = 0;

        loop {
            if total_tool_calls >= self.config.max_tool_calls
                || loop_iterations >= self.config.max_tool_calls
            {
                let violations = self.safety_guard.check_tool_calls(total_tool_calls);
                if violations.is_empty() {
                    self.trajectory.record_step(
                        TrajectoryAction::SafetyIntervention {
                            check_type: "MaxToolCalls".to_string(),
                            violation: format!(
                                "Reached max tool calls: {}",
                                total_tool_calls
                            ),
                            severity: "Blocking".to_string(),
                        },
                        None,
                    );
                } else {
                    for v in &violations {
                        self.trajectory.record_step(
                            TrajectoryAction::SafetyIntervention {
                                check_type: format!("{:?}", v.rule),
                                violation: v.message.clone(),
                                severity: format!("{:?}", v.severity),
                            },
                            None,
                        );
                    }
                }
                warn!(
                    "Max tool calls reached: {} / {}",
                    total_tool_calls, self.config.max_tool_calls
                );
                break;
            }

            loop_iterations += 1;

            self.conversation.maybe_compress(provider.as_ref()).await;

            let tool_defs: Option<Vec<ToolDefinition>> = if tool_definitions.is_empty() {
                None
            } else {
                Some(tool_definitions.clone())
            };
            let tool_defs_ref: Option<&[ToolDefinition]> = tool_defs.as_deref();

            let response = match self
                .conversation
                .next_response(provider.as_ref(), tool_defs_ref)
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    warn!("LLM provider error: {e}");
                    self.trajectory.record_step(
                        TrajectoryAction::Thought {
                            content: format!("[error: {e}]"),
                        },
                        None,
                    );
                    break;
                }
            };

            match response {
                ConversationResponse::Text { content } => {
                    info!("Agent thought ({} chars)", content.len());
                    self.trajectory.record_step(
                        TrajectoryAction::Thought {
                            content: content.clone(),
                        },
                        Some(content.clone()),
                    );

                    if content.contains(CHAPTER_COMPLETE_MARKER) {
                        info!("Agent signaled chapter completion");

                        let pre_safety_context = SafetyCheckContext {
                            current_tool_calls: total_tool_calls,
                            current_tool_call_rounds: 0,
                            current_revision_rounds: revision_rounds,
                            output_length: content.len() as u64,
                            min_output_length: 100,
                            started_at: self.trajectory.started_at,
                            tool_call_steps: total_tool_calls,
                            total_steps: self.trajectory.steps.len() as u32,
                        };
                        let pre_report = self.safety_guard.check_all(&pre_safety_context);

                        if pre_report.has_blocking {
                            for violation in pre_report.blocking_violations() {
                                self.trajectory.record_step(
                                    TrajectoryAction::SafetyIntervention {
                                        check_type: format!("{:?}", violation.rule),
                                        violation: violation.message.clone(),
                                        severity: "blocking".to_string(),
                                    },
                                    None,
                                );
                            }
                            let feedback = format!(
                                "Your chapter completion was blocked by safety checks:\n{}",
                                pre_report
                                    .blocking_violations()
                                    .iter()
                                    .map(|v| format!("- {}", v.message))
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            );
                            self.conversation
                                .append_message(Message::user(&feedback));
                            warn!(
                                "Chapter completion blocked by safety, continuing loop"
                            );
                            continue;
                        }

                        break;
                    }
                }

                ConversationResponse::ToolCalls { calls } => {
                    info!("Agent requested {} tool call(s)", calls.len());

                    for call in &calls {
                        total_tool_calls += 1;

                        let tool_start = Instant::now();
                        let tool_result = self
                            .tool_registry
                            .execute(&call.function_name, &tool_context, call.arguments.clone())
                            .await;

                        let tool_duration_ms = tool_start.elapsed().as_millis() as u64;

                        match tool_result {
                            Ok(output) => {
                                let result_content = output.content.clone();
                                let is_error = output.is_error;

                                self.trajectory.record_step(
                                    TrajectoryAction::ToolCall {
                                        tool_name: call.function_name.clone(),
                                        arguments: call.arguments.clone(),
                                        result: Some(result_content.clone()),
                                        duration_ms: tool_duration_ms,
                                    },
                                    None,
                                );

                                self.conversation.inject_tool_result(
                                    &call.id,
                                    &result_content,
                                    is_error,
                                );
                            }
                            Err(e) => {
                                let error_msg = format!("tool execution failed: {e}");
                                warn!("{}", error_msg);

                                self.trajectory.record_step(
                                    TrajectoryAction::ToolCall {
                                        tool_name: call.function_name.clone(),
                                        arguments: call.arguments.clone(),
                                        result: Some(error_msg.clone()),
                                        duration_ms: tool_duration_ms,
                                    },
                                    None,
                                );

                                self.conversation
                                    .inject_tool_result(&call.id, &error_msg, true);
                            }
                        }

                        let violations =
                            self.safety_guard.check_tool_calls(total_tool_calls);
                        let has_blocking = violations
                            .iter()
                            .any(|v| v.severity == SafetySeverity::Blocking);

                        for v in &violations {
                            self.trajectory.record_step(
                                TrajectoryAction::SafetyIntervention {
                                    check_type: format!("{:?}", v.rule),
                                    violation: v.message.clone(),
                                    severity: format!("{:?}", v.severity),
                                },
                                None,
                            );
                        }

                        if has_blocking {
                            warn!(
                                "Blocking safety violation at tool call #{}",
                                total_tool_calls
                            );
                            break;
                        }

                        if total_tool_calls >= self.config.max_tool_calls {
                            break;
                        }
                    }

                    if total_tool_calls >= self.config.max_tool_calls {
                        let force_violations =
                            self.safety_guard.check_tool_calls(total_tool_calls);
                        if force_violations.is_empty() {
                            self.trajectory.record_step(
                                TrajectoryAction::SafetyIntervention {
                                    check_type: "MaxToolCalls".to_string(),
                                    violation: format!(
                                        "Reached max tool calls: {}",
                                        total_tool_calls
                                    ),
                                    severity: "Blocking".to_string(),
                                },
                                None,
                            );
                        } else {
                            for v in &force_violations {
                                self.trajectory.record_step(
                                    TrajectoryAction::SafetyIntervention {
                                        check_type: format!("{:?}", v.rule),
                                        violation: v.message.clone(),
                                        severity: format!("{:?}", v.severity),
                                    },
                                    None,
                                );
                            }
                        }
                        warn!(
                            "Max tool calls reached: {} / {}",
                            total_tool_calls, self.config.max_tool_calls
                        );
                        break;
                    }
                }
            }

            revision_rounds += 1;
            if revision_rounds >= self.config.max_revision_rounds * self.config.max_tool_calls {
                warn!("Max revision rounds reached: {}", revision_rounds);
                break;
            }
        }

        let chapter_content = self.extract_chapter_content();
        let output_length = chapter_content.as_ref().map(|s| s.len() as u64).unwrap_or(0);

        let safety_context = SafetyCheckContext {
            current_tool_calls: total_tool_calls,
            current_tool_call_rounds: 0,
            current_revision_rounds: revision_rounds,
            output_length,
            min_output_length: 100,
            started_at: self.trajectory.started_at,
            tool_call_steps: total_tool_calls,
            total_steps: self.trajectory.steps.len() as u32,
        };

        let safety_report = self.safety_guard.check_all(&safety_context);
        let final_verdict = if safety_report.has_blocking {
            FinalVerdict::Rejected {
                violations: safety_report.blocking_violations().into_iter().cloned().collect(),
            }
        } else if safety_report.has_warnings {
            FinalVerdict::ApprovedWithWarnings {
                warnings: safety_report.warning_violations().into_iter().cloned().collect(),
            }
        } else {
            FinalVerdict::Approved
        };

        let word_count = chapter_content
            .as_ref()
            .map(|s| s.split_whitespace().count() as u64)
            .unwrap_or(0);

        self.trajectory.record_step(
            TrajectoryAction::ChapterComplete {
                chapter_number: self.config.chapter_number,
                word_count,
                verdict: Some(final_verdict.clone()),
            },
            None,
        );

        self.trajectory.mark_completed();

        let duration_ms = start.elapsed().as_millis() as u64;
        let total_steps = self.trajectory.steps.len() as u32;

        info!(
            "Autonomous loop complete: {total_steps} steps, {total_tool_calls} tool calls, \
             {duration_ms}ms, verdict: {:?}",
            final_verdict
        );

        AutonomousResult {
            chapter_content,
            trajectory: self.trajectory.clone(),
            safety_report,
            total_tool_calls,
            total_steps,
            duration_ms,
            final_verdict,
        }
    }

    /// Runs the autonomous loop with concurrent fork execution.
    ///
    /// Spawns the provided fork tasks via [`ForkManager`], runs the main loop,
    /// then collects and merges fork results.
    pub async fn run_with_forks(
        &mut self,
        task_description: &str,
        fork_tasks: Vec<(String, Vec<Box<dyn AgentTool>>)>,
        provider: Arc<dyn LlmProvider>,
        memory: Arc<dyn MemorySystem>,
        knowledge_graph: Option<Arc<dyn KnowledgeGraph>>,
    ) -> AutonomousResult {
        let tool_context = ToolContext {
            novel_id: self.config.novel_id.clone(),
            chapter_number: self.config.chapter_number,
            provider: Arc::clone(&provider),
            memory: Arc::clone(&memory),
            knowledge_graph: knowledge_graph.clone(),
        };

        if let Some(ref mut fm) = self.fork_manager {
            if !fork_tasks.is_empty() {
                info!("Spawning {} fork(s)", fork_tasks.len());
                for (desc, tools) in fork_tasks {
                    fm.spawn_fork(&desc, tools, tool_context.clone());
                }
            }
        } else {
            info!("Forking disabled, skipping fork tasks");
        }

        let result = self
            .run(task_description, Arc::clone(&provider), Arc::clone(&memory), knowledge_graph)
            .await;

        if let Some(ref mut fm) = self.fork_manager {
            if fm.pending_count() > 0 {
                info!("Running {} pending fork(s)", fm.pending_count());
                let fork_run_results = fm
                    .run_all_forks(provider.as_ref(), &self.tool_registry)
                    .await;

                for fr in fork_run_results {
                    match fr {
                        Ok(fork_result) => {
                            info!("Fork {} completed successfully", fork_result.fork_id);
                        }
                        Err(e) => {
                            warn!("Fork failed: {e}");
                        }
                    }
                }

                let _ = fm.collect_results();
            }
        }

        result
    }

    /// Returns a reference to the execution trajectory.
    pub fn get_trajectory(&self) -> &AgentTrajectory {
        &self.trajectory
    }

    /// Returns a reference to the conversation manager.
    pub fn get_conversation(&self) -> &ConversationManager {
        &self.conversation
    }

    /// Returns the latest safety report, if available.
    ///
    /// The report is generated during [`run`](Self::run) and stored in
    /// [`AutonomousResult`]. This returns `None` since the loop does not
    /// cache the report across runs.
    pub fn get_safety_report(&self) -> Option<&SafetyReport> {
        None
    }

    /// Builds a dynamic system prompt containing the task description,
    /// available tools, and safety constraints.
    ///
    /// v2 — Optimized for chapter creation with a six-phase workflow
    /// (Think → Query → Draft → Review → Revise → Finalize), tool
    /// descriptions grouped by category, explicit formatting guidance,
    /// and safety constraints with rationale.
    fn build_system_prompt(&self, task_description: &str) -> String {
        let tool_definitions = self.tool_registry.list_definitions();
        let mut prompt = String::new();

        // ── Role ──
        prompt.push_str(
            "You are a professional novel-writing agent in the XiaoZhu (小竹) ecosystem. \
             Your purpose is to draft and refine narrative content \
             (novel chapters, story arcs, character-driven scenes) \
             using a structured workflow.\n\n",
        );

        // ── Task ──
        prompt.push_str(&format!("## Task\n{task_description}\n\n"));

        // ── Tools ──
        prompt.push_str("## Available Tools\n\n");
        if tool_definitions.is_empty() {
            prompt.push_str("No tools are currently registered.\n\n");
        } else {
            // Group tools by naming convention into logical categories.
            // This is a heuristic — tool names that contain "search" or "fetch"
            // are information-gathering; names that contain "write" or "save"
            // are content operations; everything else is general-purpose.
            let mut info_tools: Vec<&ToolDefinition> = Vec::new();
            let mut content_tools: Vec<&ToolDefinition> = Vec::new();
            let mut other_tools: Vec<&ToolDefinition> = Vec::new();

            for td in &tool_definitions {
                let name_lower = td.name.to_lowercase();
                if name_lower.contains("search") || name_lower.contains("fetch") || name_lower.contains("query") {
                    info_tools.push(td);
                } else if name_lower.contains("write") || name_lower.contains("save") || name_lower.contains("create") {
                    content_tools.push(td);
                } else {
                    other_tools.push(td);
                }
            }

            if !info_tools.is_empty() {
                prompt.push_str("### Information Gathering\n");
                prompt.push_str("Use these to look up facts, research details, or retrieve context.\n\n");
                for td in &info_tools {
                    prompt.push_str(&format!("- **{}**: {}\n", td.name, td.description));
                }
                prompt.push('\n');
            }
            if !content_tools.is_empty() {
                prompt.push_str("### Content Operations\n");
                prompt.push_str("Use these to write, persist, or compare chapter drafts.\n\n");
                for td in &content_tools {
                    prompt.push_str(&format!("- **{}**: {}\n", td.name, td.description));
                }
                prompt.push('\n');
            }
            if !other_tools.is_empty() {
                prompt.push_str("### General\n\n");
                for td in &other_tools {
                    prompt.push_str(&format!("- **{}**: {}\n", td.name, td.description));
                }
                prompt.push('\n');
            }
        }

        // ── Tool usage guidance ──
        prompt.push_str("### Tool Usage Formatting\n\n");
        prompt.push_str("- Call tools by name with their required parameters.\n");
        prompt.push_str("- Always verify tool results before acting on them.\n");
        prompt.push_str(&format!(
            "- Tool calls consume one of your {} allowed invocations; plan ahead.\n",
            self.config.max_tool_calls
        ));
        prompt.push_str("- Do not call the same tool with identical parameters more than once.\n\n");

        // ── Workflow ──
        prompt.push_str("## Workflow\n\n");
        prompt.push_str("Follow this six-phase cycle for every chapter:\n\n");
        prompt.push_str("1. **Think** — Read the task carefully. Identify what the chapter needs: plot beats, character moments, tone, length. Plan your approach before writing.\n");
        prompt.push_str("2. **Query** — Use information-gathering tools to retrieve context (character profiles, previous chapter summaries, world details) that will inform your draft.\n");
        prompt.push_str("3. **Draft** — Write the chapter prose. Focus on narrative flow, character voice, and scene structure. Do not self-censor during drafting.\n");
        prompt.push_str("4. **Review** — Re-read your draft against the task requirements. Check for consistency with established characters and plot. Identify passages that need improvement.\n");
        prompt.push_str("5. **Revise** — Apply the improvements identified during review. Polish prose, tighten pacing, and fix inconsistencies. You may loop between Review and Revise as needed.\n");
        prompt.push_str("6. **Finalize** — Once the chapter meets all quality standards, signal completion (see Completion below).\n\n");

        // ── Safety constraints with rationale ──
        prompt.push_str("## Safety Constraints\n\n");
        prompt.push_str(&format!(
            "- **Tool call budget**: You may invoke tools up to {} times. \
             Each invocation counts regardless of success or failure. \
             This prevents runaway loops and conserves compute resources.\n",
            self.config.max_tool_calls
        ));
        prompt.push_str(&format!(
            "- **Revision budget**: You may retry after safety warnings up to {} times. \
             If a chapter is repeatedly flagged, revisit the task requirements \
             rather than making cosmetic changes.\n",
            self.config.max_revision_rounds
        ));
        prompt.push_str(
            "- **Quality floor**: Every output must be coherent, grammatically correct, \
             and aligned with the task. Vague or placeholder content is not acceptable.\n\n",
        );

        // ── Completion signal ──
        prompt.push_str("## Completion\n\n");
        prompt.push_str(&format!(
            "When the chapter has passed your final review and is ready for \
             automated safety review, include the exact phrase \
             \"{CHAPTER_COMPLETE_MARKER}\" in your response. \
             Place it on its own line at the end of your final message. \
             Do not include it in partial or intermediate responses.\n"
        ));

        prompt
    }

    /// Extracts chapter content from the conversation history.
    ///
    /// Collects all assistant text responses (excluding tool call messages)
    /// and joins them into a single string.
    fn extract_chapter_content(&self) -> Option<String> {
        let messages = self.conversation.get_messages();
        let parts: Vec<&str> = messages
            .iter()
            .filter_map(|m| match m {
                Message::Assistant {
                    content,
                    tool_calls,
                    ..
                } => {
                    if tool_calls.as_ref().map_or(true, |tc| tc.is_empty()) {
                        match content {
                            MessageContent::Text(t) => Some(t.as_str()),
                            _ => None,
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect();

        if parts.is_empty() {
            None
        } else {
            Some(parts.join("\n\n"))
        }
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::{json, Value};
    use crate::error::AgentError;
    use crate::tool::ToolOutput;
    use xz_provider::{
        CompletionRequest, CompletionResponse, FinishReason, ModelInfo, RequestOptions,
        StreamEvent, TokenUsage, ToolCall,
    };

    struct MockProvider {
        responses: Vec<CompletionResponse>,
        current: tokio::sync::Mutex<usize>,
    }

    impl MockProvider {
        fn new(responses: Vec<CompletionResponse>) -> Self {
            Self {
                responses,
                current: tokio::sync::Mutex::new(0),
            }
        }
    }

    impl std::fmt::Debug for MockProvider {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("MockProvider")
                .field("responses", &self.responses.len())
                .finish()
        }
    }

    #[async_trait]
    impl LlmProvider for MockProvider {
        async fn complete(
            &self,
            _request: CompletionRequest,
            _options: RequestOptions,
        ) -> Result<CompletionResponse, xz_provider::ProviderError> {
            let mut cur = self.current.lock().await;
            let idx = *cur;
            if idx < self.responses.len() {
                *cur += 1;
                Ok(self.responses[idx].clone())
            } else {
                Ok(CompletionResponse {
                    content: Some("[mock: exhausted]".to_string()),
                    thinking: None,
                    tool_calls: vec![],
                    usage: TokenUsage::default(),
                    model: "mock".to_string(),
                    finish_reason: FinishReason::Stop,
                    latency_ms: 1,
                    cache_info: None,
                })
            }
        }

        async fn complete_stream(
            &self,
            _request: CompletionRequest,
            _options: RequestOptions,
        ) -> Result<
            std::pin::Pin<
                Box<
                    dyn futures::Stream<Item = Result<StreamEvent, xz_provider::ProviderError>>
                        + Send,
                >,
            >,
            xz_provider::ProviderError,
        > {
            unimplemented!("streaming not used in autonomous tests")
        }

        fn models(&self) -> &[ModelInfo] {
            &[]
        }

        fn name(&self) -> &str {
            "mock"
        }
    }

    fn make_test_loop() -> AutonomousLoop {
        let config = AutonomousConfig {
            novel_id: "test-novel".into(),
            chapter_number: 1,
            ..AutonomousConfig::default()
        };
        let registry = Arc::new(ToolRegistry::new());
        AutonomousLoop::new(config, registry)
    }

    fn text_response(content: &str) -> CompletionResponse {
        CompletionResponse {
            content: Some(content.to_string()),
            thinking: None,
            tool_calls: vec![],
            usage: TokenUsage::default(),
            model: "mock".to_string(),
            finish_reason: FinishReason::Stop,
            latency_ms: 1,
            cache_info: None,
        }
    }

    fn tool_call_response(calls: Vec<ToolCall>) -> CompletionResponse {
        CompletionResponse {
            content: None,
            thinking: None,
            tool_calls: calls,
            usage: TokenUsage::default(),
            model: "mock".to_string(),
            finish_reason: FinishReason::ToolCall,
            latency_ms: 1,
            cache_info: None,
        }
    }

    fn mock_tool_output() -> ToolOutput {
        ToolOutput {
            content: "mock result".to_string(),
            structured: None,
            is_error: false,
            tool_call_id: None,
        }
    }

    #[tokio::test]
    async fn test_autonomous_loop_text_only() {
        let provider = Arc::new(MockProvider::new(vec![
            text_response("I am thinking about the chapter."),
            text_response("Let me refine the plot."),
            text_response("This looks great. CHAPTER_COMPLETE"),
        ]));

        let mut loop_ = make_test_loop();
        let memory = Arc::new(xz_memory::InMemoryMemory::new());
        let result = loop_
            .run("Write a test chapter", provider, memory, None)
            .await;

        assert!(result.total_steps > 0);
        assert_eq!(result.total_tool_calls, 0);
        assert!(
            result
                .trajectory
                .steps
                .iter()
                .any(|s| matches!(s.action, TrajectoryAction::Thought { .. }))
        );
        assert!(
            result
                .trajectory
                .steps
                .iter()
                .any(|s| matches!(s.action, TrajectoryAction::ChapterComplete { .. }))
        );
        assert!(result.chapter_content.is_some());
    }

    #[tokio::test]
    async fn test_autonomous_loop_tool_call() {
        let calls = vec![ToolCall {
            id: "call-1".to_string(),
            function_name: "mock_search".to_string(),
            arguments: json!({"q": "test"}),
        }];

        let provider = Arc::new(MockProvider::new(vec![
            tool_call_response(calls),
            text_response("Got the info. CHAPTER_COMPLETE"),
        ]));

        let mut registry = ToolRegistry::new();
        struct MockTool;
        #[async_trait]
        impl AgentTool for MockTool {
            fn name(&self) -> &str {
                "mock_search"
            }
            fn description(&self) -> &str {
                "A mock search tool"
            }
            fn parameter_schema(&self) -> Value {
                json!({"type": "object"})
            }
            async fn execute(
                &self,
                _context: &ToolContext,
                _args: Value,
            ) -> Result<ToolOutput, AgentError> {
                Ok(mock_tool_output())
            }
        }
        registry.register(Box::new(MockTool)).unwrap();

        let config = AutonomousConfig {
            novel_id: "test-novel".into(),
            chapter_number: 1,
            ..AutonomousConfig::default()
        };

        let mut loop_ = AutonomousLoop::new(config, Arc::new(registry));
        let memory = Arc::new(xz_memory::InMemoryMemory::new());
        let result = loop_
            .run("Write a test chapter", provider, memory, None)
            .await;

        assert_eq!(result.total_tool_calls, 1);
        assert!(
            result
                .trajectory
                .steps
                .iter()
                .any(|s| matches!(s.action, TrajectoryAction::ToolCall { .. }))
        );
        assert!(
            result
                .trajectory
                .steps
                .iter()
                .any(|s| matches!(s.action, TrajectoryAction::ChapterComplete { .. }))
        );
    }

    #[tokio::test]
    async fn test_autonomous_loop_safety_rejection() {
        let calls = vec![ToolCall {
            id: "call-1".to_string(),
            function_name: "mock_search".to_string(),
            arguments: json!({"q": "test"}),
        }];

        let provider = Arc::new(MockProvider::new(vec![
            tool_call_response(calls.clone()),
            tool_call_response(calls.clone()),
            tool_call_response(calls.clone()),
            tool_call_response(calls),
        ]));

        let mut registry = ToolRegistry::new();
        struct MockTool;
        #[async_trait]
        impl AgentTool for MockTool {
            fn name(&self) -> &str {
                "mock_search"
            }
            fn description(&self) -> &str {
                "A mock search tool"
            }
            fn parameter_schema(&self) -> Value {
                json!({"type": "object"})
            }
            async fn execute(
                &self,
                _context: &ToolContext,
                _args: Value,
            ) -> Result<ToolOutput, AgentError> {
                Ok(mock_tool_output())
            }
        }
        registry.register(Box::new(MockTool)).unwrap();

        let config = AutonomousConfig {
            novel_id: "test-novel".into(),
            chapter_number: 1,
            max_tool_calls: 1,
            ..AutonomousConfig::default()
        };

        let mut loop_ = AutonomousLoop::new(config, Arc::new(registry));
        let memory = Arc::new(xz_memory::InMemoryMemory::new());
        let result = loop_
            .run("Write a test chapter", provider, memory, None)
            .await;

        assert!(result.total_tool_calls >= 1);
        let has_safety_intervention = result
            .trajectory
            .steps
            .iter()
            .any(|s| matches!(s.action, TrajectoryAction::SafetyIntervention { .. }));
        assert!(
            has_safety_intervention,
            "expected at least one safety intervention"
        );
    }

    #[tokio::test]
    async fn test_autonomous_loop_chapter_complete() {
        let provider = Arc::new(MockProvider::new(vec![
            text_response("Let me write the chapter."),
            text_response("Here is the full chapter text. CHAPTER_COMPLETE"),
        ]));

        let mut loop_ = make_test_loop();
        let memory = Arc::new(xz_memory::InMemoryMemory::new());
        let result = loop_
            .run("Write a test chapter", provider, memory, None)
            .await;

        let has_chapter_complete = result
            .trajectory
            .steps
            .iter()
            .any(|s| matches!(s.action, TrajectoryAction::ChapterComplete { .. }));
        assert!(
            has_chapter_complete,
            "expected a ChapterComplete step in trajectory"
        );
        assert!(result.final_verdict.is_approved());
    }

    #[tokio::test]
    async fn test_autonomous_loop_retry_after_warning() {
        let provider = Arc::new(MockProvider::new(vec![
            text_response("First thought."),
            text_response("CHAPTER_COMPLETE"),
        ]));

        let mut loop_ = make_test_loop();
        let memory = Arc::new(xz_memory::InMemoryMemory::new());
        let result = loop_
            .run("Write a test chapter", provider, memory, None)
            .await;

        assert!(
            result.final_verdict.is_approved(),
            "expected approved verdict, got: {:?}",
            result.final_verdict
        );
        assert!(result.total_steps >= 2);
    }

    #[tokio::test]
    async fn test_max_tool_calls_exceeded() {
        let provider = Arc::new(MockProvider::new(vec![
            text_response("Thinking about the chapter."),
            text_response("Still thinking..."),
            text_response("Not done yet."),
        ]));

        let config = AutonomousConfig {
            novel_id: "test-novel".into(),
            chapter_number: 1,
            max_tool_calls: 2,
            ..AutonomousConfig::default()
        };
        let registry = Arc::new(ToolRegistry::new());
        let mut loop_ = AutonomousLoop::new(config, registry);
        let memory = Arc::new(xz_memory::InMemoryMemory::new());
        let result = loop_
            .run("Write a test chapter", provider, memory, None)
            .await;

        assert!(
            result.total_steps >= 2,
            "expected at least 2 steps, got {}",
            result.total_steps
        );
        assert_eq!(result.total_tool_calls, 0);
        // Verify trajectory was properly marked as completed
        assert!(result.trajectory.completed_at.is_some());
        // Forced completion should have a ChapterComplete step
        let has_chapter_complete = result
            .trajectory
            .steps
            .iter()
            .any(|s| matches!(s.action, TrajectoryAction::ChapterComplete { .. }));
        assert!(
            has_chapter_complete,
            "expected ChapterComplete for forced completion"
        );
    }

    #[test]
    fn test_build_system_prompt() {
        let config = AutonomousConfig {
            model: "test-model".into(),
            max_tool_calls: 10,
            max_revision_rounds: 5,
            ..AutonomousConfig::default()
        };

        let mut registry = ToolRegistry::new();
        struct MockPromptTool {
            name: &'static str,
            desc: &'static str,
        }
        #[async_trait]
        impl AgentTool for MockPromptTool {
            fn name(&self) -> &str { self.name }
            fn description(&self) -> &str { self.desc }
            fn parameter_schema(&self) -> Value { json!({"type": "object"}) }
            async fn execute(&self, _: &ToolContext, _: Value) -> Result<ToolOutput, AgentError> {
                Ok(ToolOutput {
                    content: "ok".into(),
                    structured: None,
                    is_error: false,
                    tool_call_id: None,
                })
            }
        }
        registry.register(Box::new(MockPromptTool { name: "search", desc: "Search the web" })).unwrap();
        registry.register(Box::new(MockPromptTool { name: "fetch", desc: "Fetch a URL" })).unwrap();

        let loop_ = AutonomousLoop::new(config.clone(), Arc::new(registry));
        let prompt = loop_.build_system_prompt("Write chapter 5");

        assert!(prompt.contains("Write chapter 5"));
        assert!(prompt.contains("**search**: Search the web"));
        assert!(prompt.contains("**fetch**: Fetch a URL"));
        assert!(prompt.contains(CHAPTER_COMPLETE_MARKER));
        assert!(prompt.contains("10"));
        assert!(prompt.contains("5"));
    }

    #[test]
    fn test_autonomous_config_default() {
        let config = AutonomousConfig::default();
        assert_eq!(config.model, "gpt-4o");
        assert_eq!(config.max_tool_calls, 50);
        assert_eq!(config.max_revision_rounds, 5);
        assert!(!config.fork_enabled);
        assert_eq!(config.max_concurrent_forks, 3);
        assert_eq!(config.chapter_number, 1);
        assert_eq!(config.novel_id, "default-novel");
        assert!(config.temperature.is_none());
        assert!(config.max_tokens.is_none());
    }
}
