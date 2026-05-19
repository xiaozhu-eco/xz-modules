# AutonomousLoop Implementation Learnings

## Patterns
- `SafetyGuard.check_tool_calls()` uses strict `>` comparison, so `current == threshold` does NOT trigger a blocking violation
- The top-of-loop force-complete check and ToolCalls branch breaks both need to record SafetyIntervention steps before breaking
- `build_system_prompt` should be a `&self` method to access `self.tool_registry.list_definitions()` and `self.config`
- Conversation starting message should include "Begin work on: {task_description}" prefix

## Key Fixes Made
1. Added `loop_iterations` counter for text-only force-complete scenarios
2. CHAPTER_COMPLETE detection now runs pre-safety-check; if blocking, injects LLM feedback and continues loop
3. ToolCalls branch records SafetyIntervention before breaking on max_tool_calls exceeded
4. `SafetyReport` now derives `Serialize, Deserialize` to support `AutonomousResult` serialization
5. `AutonomousConfig.max_revision_rounds` default fixed from 3 to 5

## T14: E2E Test Learnings

### Patterns
- `InMemoryMemory` is always exported from `xz-memory` (not gated behind `test-utils` feature despite doc comment)
- `ToolCall` has no explicit constructor — use struct literal syntax directly
- `CompletionResponse` fields: use `finish_reason: FinishReason::ToolCall` for tool call responses and `FinishReason::Stop` for text
- MockProvider pattern from existing unit tests can be reused: `tokio::sync::Mutex<usize>` counter for call sequencing

### Test Flow
- Each `provider.complete()` call is one loop iteration
- When LLM returns ToolCalls, autonomous loop executes them, injects results, then loops back for next completion call
- CHAPTER_COMPLETE detection happens on text responses containing the marker string
- After the loop breaks, AutonomousLoop records a ChapterComplete step and runs final safety checks

### Safety Test Nuances
- `SafetyGuard.check_tool_calls()` uses `current > threshold` (strict), so with `max_tool_calls=1`, violation only fires at 2+
- The MinOutputLength warning (threshold 100 chars) fires when no chapter was completed
- Trajectory still gets SafetyIntervention steps from the loop-level force-complete logic

## T42: System Prompt Optimization (v2)

### Changes Made
- Upgraded `build_system_prompt` from v1 to v2 with a version doc comment
- Six-phase workflow: Think -> Query -> Draft -> Review -> Revise -> Finalize (replaces flat 4-step)
- Tools now grouped by naming heuristic: Information Gathering (search/fetch/query), Content Operations (write/save/create), General (everything else)
- Added explicit Tool Usage Formatting section with usage guidance
- Safety constraints now include rationale ("why" each limit exists)
- Completion signal guidance: marker must be on its own line at end of final message, not in partial responses

### Patterns
- `ToolDefinition` lives in `xz_provider::ToolDefinition`, already imported at top of file -- use bare `ToolDefinition` not `crate::tool::ToolDefinition`
- Test assertions check for substring presence: `"- **search**: Search the web"` contains `"**search**: Search the web"`
- The function signature and test interface must remain unchanged since tests depend on `&self` accessor

### Pre-existing Issues
- `SafetyCheckContext` missing `current_tool_call_rounds` field -- blocks compilation of autonomous module entirely
- Integration tests in `xz-agent/tests/agent_memory_integration.rs` have import errors (unrelated to prompt work)
