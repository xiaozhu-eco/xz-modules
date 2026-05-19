//! End-to-end tests for the [`AutonomousLoop`] using mock provider and mock tools.
//!
//! These tests simulate a complete chapter creation workflow, exercising the
//! full Think → Act → Verify → Complete loop with trajectory recording and
//! safety checks.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};
use xz_agent::*;
use xz_provider::{
    CompletionRequest, CompletionResponse, FinishReason, LlmProvider, ModelInfo, RequestOptions,
    StreamEvent, TokenUsage, ToolCall,
};

// ── MockProvider ──

/// A mock LLM provider that returns controlled responses based on call count.
///
/// Each call returns the next response in the pre-configured sequence.
/// Once exhausted, returns a dummy text response.
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
            .field("response_count", &self.responses.len())
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
            Box<dyn futures::Stream<Item = Result<StreamEvent, xz_provider::ProviderError>> + Send>,
        >,
        xz_provider::ProviderError,
    > {
        unimplemented!("streaming not used in e2e tests")
    }

    fn models(&self) -> &[ModelInfo] {
        &[]
    }

    fn name(&self) -> &str {
        "mock"
    }
}

// ── Response builders ──

/// Build a plain text [`CompletionResponse`].
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

/// Build a tool-call [`CompletionResponse`].
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

// ── Mock Tools ──

/// Mock tool that simulates querying character information.
struct MockQueryTool;

#[async_trait]
impl AgentTool for MockQueryTool {
    fn name(&self) -> &str {
        "query_characters"
    }

    fn description(&self) -> &str {
        "Queries character information from the knowledge base"
    }

    fn parameter_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "character_name": {"type": "string"}
            }
        })
    }

    async fn execute(
        &self,
        _context: &ToolContext,
        _args: Value,
    ) -> Result<ToolOutput, AgentError> {
        Ok(ToolOutput {
            content: "Found character: Maria (protagonist, age 28, journalist)".to_string(),
            structured: Some(json!({
                "name": "Maria",
                "role": "protagonist",
                "age": 28,
                "occupation": "journalist"
            })),
            is_error: false,
            tool_call_id: None,
        })
    }
}

/// Mock tool that simulates drafting a scene.
struct MockCreateTool;

#[async_trait]
impl AgentTool for MockCreateTool {
    fn name(&self) -> &str {
        "draft_scene"
    }

    fn description(&self) -> &str {
        "Drafts a scene based on the provided scene plan"
    }

    fn parameter_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "scene_plan": {"type": "string"}
            }
        })
    }

    async fn execute(
        &self,
        _context: &ToolContext,
        _args: Value,
    ) -> Result<ToolOutput, AgentError> {
        Ok(ToolOutput {
            content: "Scene drafted successfully — 450 words generated".to_string(),
            structured: None,
            is_error: false,
            tool_call_id: None,
        })
    }
}

/// Mock tool that simulates a consistency review.
struct MockReviewTool;

#[async_trait]
impl AgentTool for MockReviewTool {
    fn name(&self) -> &str {
        "check_consistency"
    }

    fn description(&self) -> &str {
        "Checks for internal consistency across characters, plot points, and timeline"
    }

    fn parameter_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "scope": {"type": "string"}
            }
        })
    }

    async fn execute(
        &self,
        _context: &ToolContext,
        _args: Value,
    ) -> Result<ToolOutput, AgentError> {
        Ok(ToolOutput {
            content: "No consistency issues found — all character arcs and plot points align".to_string(),
            structured: None,
            is_error: false,
            tool_call_id: None,
        })
    }
}

// ── Helper: build a ToolCall ──

fn make_tool_call(id: &str, function_name: &str, args: Value) -> ToolCall {
    ToolCall {
        id: id.to_string(),
        function_name: function_name.to_string(),
        arguments: args,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════

/// End-to-end test simulating a complete chapter creation workflow.
///
/// The mock provider sequence:
/// 1. ToolCalls → query_characters (simulate info gathering)
/// 2. Text → "Analyzing the plot..." (simulate thinking)
/// 3. ToolCalls → draft_scene (simulate creation)
/// 4. Text → "CHAPTER_COMPLETE — The chapter is ready" (simulate completion)
///
/// This exercises the full Think → Act → Verify → Complete cycle.
#[tokio::test]
async fn test_e2e_chapter_creation() {
    // ── Build provider with a 4-step sequence ──
    let provider = Arc::new(MockProvider::new(vec![
        // Step 1: Query character information
        tool_call_response(vec![make_tool_call(
            "call-q1",
            "query_characters",
            json!({"character_name": "Maria"}),
        )]),
        // Step 2: Analyze and plan
        text_response("Analyzing the plot and character dynamics for this scene..."),
        // Step 3: Draft the scene
        tool_call_response(vec![make_tool_call(
            "call-d1",
            "draft_scene",
            json!({"scene_plan": "Opening scene with Maria at the newsroom"}),
        )]),
        // Step 4: Signal completion
        text_response("CHAPTER_COMPLETE — The chapter is ready with a compelling opening scene."),
    ]));

    // ── Build tool registry with mock tools ──
    let mut registry = ToolRegistry::new();
    registry
        .register(Box::new(MockQueryTool))
        .expect("register query tool");
    registry
        .register(Box::new(MockCreateTool))
        .expect("register create tool");
    registry
        .register(Box::new(MockReviewTool))
        .expect("register review tool");

    // ── Build AutonomousLoop ──
    let config = AutonomousConfig {
        novel_id: "novel-e2e-1".into(),
        chapter_number: 1,
        max_tool_calls: 10,
        max_revision_rounds: 3,
        ..AutonomousConfig::default()
    };
    let mut loop_ = AutonomousLoop::new(config, Arc::new(registry));

    // ── Run ──
    let memory = Arc::new(xz_memory::InMemoryMemory::new());
    let result = loop_
        .run(
            "Write the opening scene introducing protagonist Maria, a journalist.",
            provider,
            memory,
            None,
        )
        .await;

    // ── Verify basic result structure ──
    assert!(result.chapter_content.is_some(), "chapter content should exist");

    assert_eq!(
        result.total_tool_calls, 2,
        "exactly 2 tool calls executed"
    );

    // ── Verify trajectory has at least 4 steps ──
    assert!(
        result.trajectory.steps.len() >= 4,
        "trajectory should have at least 4 steps, got {}",
        result.trajectory.steps.len()
    );

    // ── Verify trajectory contains the expected step types ──
    let has_thought = result
        .trajectory
        .steps
        .iter()
        .any(|s| matches!(s.action, TrajectoryAction::Thought { .. }));
    assert!(has_thought, "trajectory must include Thought steps");

    let has_tool_call = result
        .trajectory
        .steps
        .iter()
        .any(|s| matches!(s.action, TrajectoryAction::ToolCall { .. }));
    assert!(has_tool_call, "trajectory must include ToolCall steps");

    let has_chapter_complete = result
        .trajectory
        .steps
        .iter()
        .any(|s| matches!(s.action, TrajectoryAction::ChapterComplete { .. }));
    assert!(
        has_chapter_complete,
        "trajectory must include a ChapterComplete step"
    );

    // ── Verify the CHAPTER_COMPLETE triggering thought ──
    let complete_thought = result
        .trajectory
        .steps
        .iter()
        .find(|s| {
            matches!(&s.action, TrajectoryAction::Thought { content }
                if content.contains("CHAPTER_COMPLETE"))
        });
    assert!(
        complete_thought.is_some(),
        "should have a thought step containing CHAPTER_COMPLETE marker"
    );

    // ── Verify the final verdict ──
    assert!(
        result.final_verdict.is_approved(),
        "final verdict should be Approved or ApprovedWithWarnings, got {:?}",
        result.final_verdict
    );

    // ── Verify total_steps is consistent ──
    assert_eq!(
        result.total_steps,
        result.trajectory.steps.len() as u32,
        "total_steps should match trajectory step count"
    );
}

/// Verify that the trajectory can be serialized to valid JSON and that
/// the human-readable display log contains expected tool names and thought content.
#[tokio::test]
async fn test_e2e_trajectory_completeness() {
    // ── Build provider ──
    let provider = Arc::new(MockProvider::new(vec![
        tool_call_response(vec![make_tool_call(
            "call-q1",
            "query_characters",
            json!({"character_name": "Maria"}),
        )]),
        text_response("Analyzing the plot structure and pacing..."),
        tool_call_response(vec![make_tool_call(
            "call-c1",
            "check_consistency",
            json!({"scope": "full_chapter"}),
        )]),
        text_response(
            "CHAPTER_COMPLETE — All scenes are consistent and well-paced.",
        ),
    ]));

    // ── Build tool registry ──
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(MockQueryTool)).unwrap();
    registry.register(Box::new(MockCreateTool)).unwrap();
    registry.register(Box::new(MockReviewTool)).unwrap();

    let config = AutonomousConfig {
        novel_id: "novel-e2e-2".into(),
        chapter_number: 2,
        max_tool_calls: 10,
        ..AutonomousConfig::default()
    };
    let mut loop_ = AutonomousLoop::new(config, Arc::new(registry));

    let memory = Arc::new(xz_memory::InMemoryMemory::new());
    let result = loop_
        .run(
            "Write the second chapter continuing from the opening scene.",
            provider,
            memory,
            None,
        )
        .await;

    // ── Verify to_json() produces valid JSON ──
    let json_str = result
        .trajectory
        .to_json()
        .expect("trajectory should serialize to JSON");
    let parsed: serde_json::Value =
        serde_json::from_str(&json_str).expect("serialized JSON should be valid");
    assert_eq!(
        parsed["session_id"].as_str(),
        Some(result.trajectory.session_id.as_str()),
        "JSON should contain the session_id"
    );
    assert!(
        parsed.get("steps").is_some(),
        "JSON should contain the steps array"
    );

    // ── Verify to_display_log() contains tool names ──
    let display_log = result.trajectory.to_display_log();
    assert!(
        display_log.contains("query_characters"),
        "display log should mention tool 'query_characters', got:\n{display_log}"
    );
    assert!(
        display_log.contains("check_consistency"),
        "display log should mention tool 'check_consistency', got:\n{display_log}"
    );

    // ── Verify to_display_log() contains thought content ──
    assert!(
        display_log.contains("Analyzing the plot structure"),
        "display log should contain the thought content, got:\n{display_log}"
    );
    assert!(
        display_log.contains("CHAPTER_COMPLETE"),
        "display log should contain CHAPTER_COMPLETE marker, got:\n{display_log}"
    );

    // ── Verify to_display_log() contains session header ──
    assert!(
        display_log.contains("Session:"),
        "display log should contain session header, got:\n{display_log}"
    );
    assert!(
        display_log.contains("Tool calls:"),
        "display log should contain tool call count, got:\n{display_log}"
    );
}

/// Safety test: when the provider only returns tool calls (never signaling
/// completion), the safety guard should force-complete and produce violations.
///
/// With max_tool_calls=1, the loop will be forced to stop after the first
/// tool call, and the safety report will contain violations due to the
/// missing chapter output.
#[tokio::test]
async fn test_e2e_safety_rejection() {
    // Always return tool calls — the loop never gets a CHAPTER_COMPLETE.
    let call = make_tool_call(
        "call-loop",
        "query_characters",
        json!({"character_name": "Maria"}),
    );

    // Provide several tool-call responses so the provider doesn't exhaust.
    let provider = Arc::new(MockProvider::new(vec![
        tool_call_response(vec![call.clone()]),
        tool_call_response(vec![call.clone()]),
        tool_call_response(vec![call.clone()]),
        tool_call_response(vec![call]),
    ]));

    let mut registry = ToolRegistry::new();
    registry.register(Box::new(MockQueryTool)).unwrap();

    let config = AutonomousConfig {
        novel_id: "novel-safety".into(),
        chapter_number: 1,
        max_tool_calls: 1, // very low — forces safety intervention
        max_revision_rounds: 1,
        ..AutonomousConfig::default()
    };
    let mut loop_ = AutonomousLoop::new(config, Arc::new(registry));

    let memory = Arc::new(xz_memory::InMemoryMemory::new());
    let result = loop_
        .run(
            "Write a chapter that the agent will never complete.",
            provider,
            memory,
            None,
        )
        .await;

    // ── Verify safety violations exist in the report ──
    assert!(
        !result.safety_report.violations.is_empty(),
        "safety report should have violations (e.g., MinOutputLength not met), \
         got {} violations",
        result.safety_report.violations.len()
    );

    // ── Verify the trajectory has SafetyIntervention steps ──
    let has_safety_intervention = result
        .trajectory
        .steps
        .iter()
        .any(|s| matches!(s.action, TrajectoryAction::SafetyIntervention { .. }));
    assert!(
        has_safety_intervention,
        "trajectory should have at least one SafetyIntervention step"
    );

    // ── Verify the chapter was NOT completed normally ──
    assert!(
        result.chapter_content.is_none()
            || result.chapter_content.as_deref() == Some(""),
        "chapter should not have been completed (no CHAPTER_COMPLETE marker)"
    );

    // ── Verify total_tool_calls is capped ──
    assert!(
        result.total_tool_calls <= 1,
        "total tool calls should not exceed the max_tool_calls limit"
    );
}
