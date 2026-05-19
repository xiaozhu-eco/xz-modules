# AWE Implementation — Learnings

## T5: ConversationManager

### Key Decisions

1. **Message type reuse**: Directly reuses `xz_provider::Message` (enum with System/User/Assistant/Tool variants) — no separate message type needed. The enum already covers all roles with proper constructors (`Message::system()`, `Message::user()`, etc.).

2. **Error conversion**: Mapped `ProviderError` → `AgentError::Io(format!("LLM provider error: {e}"))`. There's no dedicated "Internal" or "Provider" variant on `AgentError`, so `Io` serves as the closest match for wrapping external/provider errors.

3. **Compression strategy**: 
   - Preserves system message at front, keeps most recent `max_context_messages/2` messages
   - Summarises the middle block using a dedicated LLM call
   - Inserts summary as a `User` message with `[Summary of previous conversation]` prefix
   - Best-effort: failures are logged with `tracing::warn!` and the original history is preserved unchanged

4. **Mock provider for testing**: Manual mock implementing `LlmProvider` trait — avoids adding `mockall` dependency. The mock supports two modes (Text / ToolCalls) via an internal enum.

### Patterns Established

- `start()` replaces all existing messages (clears first, then adds system + user)
- `next_response()` clones messages for the request, then appends the assistant response to history after decoding
- Tool-call assistant messages use `MessageContent::None` (empty content) with `tool_calls: Some(...)` — matching the provider convention

### Known Issues

- Pre-existing integration test `test_register_and_trigger_linear_pipeline` fails (unrelated to this change)
- LSP (rust-analyzer) not available in current environment

## T15: xz-modules v0.2.0 Release

### Key Decisions

1. **Workspace version bump only**: Changed `[workspace.package] version` from `"0.1.4"` to `"0.2.0"` in root `Cargo.toml`. Individual crates inherit via `workspace = true`.

### Test Results (2026-05-19)

| Crate | Status | Count |
|-------|--------|-------|
| xz-agent | ALL PASS | 118 tests (81 unit + 7 integ + 7 trigger + 3 + 7 + 13 doc) |
| xz-memory | ALL PASS | 60 tests (26 domain + 34 other) |
| xz-provider | 2 pre-existing failures | 147 pass / 2 fail |
| All other crates | ALL PASS | — |

### Pre-existing xz-provider Failures (not caused by v0.2.0 changes)
1. `providers::sse::tests::test_sse_broken_per_chunk_lines_demo`
2. `router::tests::router_latency_persistence`

### Verification Output
- Evidence file: `.sisyphus/evidence/task-15-release-pass.txt`
- Verdict: READY FOR v0.2.0

## [2026-05-19] Wave 1-4 Complete — xz-modules v0.2.0 Released

### Completed Tasks (15/15 in xz-modules scope)

**Wave 1: xz-agent 类型层**
- T1: AgentTool trait + ToolRegistry (`xz-agent/src/tool/`)
- T2: ToolCall/ToolOutput 对接 xz-provider (`xz-agent/src/tool/types.rs`)
- T3: AgentTrajectory + TrajectoryStep (`xz-agent/src/trajectory/`)
- T4: SafetyViolation/SafetyRule types (`xz-agent/src/safety/types.rs`)

**Wave 2: xz-agent 核心**
- T5: ConversationManager (`xz-agent/src/conversation/`)
- T6: AutonomousLoop 主循环 (`xz-agent/src/autonomous/`)
- T7: SafetyGuard 规则引擎 (`xz-agent/src/safety/guard.rs`)
- T8: ForkManager (`xz-agent/src/fork/`)

**Wave 3: xz-memory 升级**
- T9: CharacterMemory (`xz-memory/src/domain/character.rs`)
- T10: SeedMemory (`xz-memory/src/domain/seed.rs`)
- T11: PlotMemory (`xz-memory/src/domain/plot.rs`)
- T12: StyleMemory (`xz-memory/src/domain/style.rs`)

**Wave 4: 集成测试 + 发布**
- T13: xz-agent + xz-memory 集成测试
- T14: AutonomousLoop 端到端测试
- T15: 发布 xz-modules v0.2.0

### Test Results
- xz-agent: 81 unit + 7 integration + 7 trigger + 13 doc = ALL PASS
- xz-memory: 26 domain + ~34 other = ALL PASS
- Total: ~168 tests pass across our changes

### Key Architecture Decisions
- ToolContext uses Arc<dyn LlmProvider>, Arc<dyn MemorySystem>, Option<Arc<dyn KnowledgeGraph>>
- ConversationManager reuses xz_provider::Message directly (no separate message type)
- AutonomousLoop integrates all subsystems: ConversationManager → ToolRegistry → SafetyGuard → AgentTrajectory
- Memory domains (Character/Seed/Plot/Style) stored as Facts via MemorySystem
- FinalVerdict defined in safety module, reused by trajectory

### Pre-existing Issues
- xz-provider has 2 pre-existing test failures (SSE line parsing, router blocking-in-async)
- Not caused by our changes

### Remaining Tasks (not in xz-modules scope)
Wave 5+: awe-tools + writer workspace (separate repo)

## T29: fork_scene_drafters + request_human_help

### Patterns
- ForkSceneDraftersTool wraps ForkManager to spawn parallel DraftSceneTool sub-agents
- Each scene gets its own fork with independent tool set (just DraftSceneTool)
- Results collected from ForkManager handles after `run_all_forks`
- RequestHumanHelpTool uses `AgentError::Paused` to signal autonomous loop

### Gotchas
- `LlmProvider` trait v2 requires `models()` to return `&[ModelInfo]` (reference), not `Vec<ModelInfo>`
- Mock implementation must use `#[async_trait]` on the impl block for trait methods to match lifetimes
- `CompletionResponse` v2 has no `id`, `created`, or `system_fingerprint` fields; uses `thinking`, `latency_ms`, `cache_info` instead
- `TokenUsage::new(prompt, completion)` constructor is available as a convenience
- `finish_reason` is now a value, not `Option<FinishReason>`

## T26: check_consistency tool

### Implementation Notes
- Created `awe-tools/src/review/check_consistency.rs`
- Updated `awe-tools/src/review/mod.rs` to add `pub mod check_consistency` and re-export `CheckConsistencyTool` + `ConsistencyIssue`

### Key Patterns Used
- `AgentTool` trait from `xz-agent`: name, description, parameter_schema, execute
- Provider call pattern: `CompletionRequest` struct literal with `..Default::default()` (clippy prefers this over `mut` + field assignment)
- `CharacterMemory` and `PlotMemory` from `xz-memory::domain`
- `EntityQuery` + `PageRequest` from `xz-knowledge-graph` for KG cross-referencing
- `Default` trait implementation for unit structs (clippy requirement)

### Verification
- 6 tests pass (test_tool_metadata, test_parameter_schema, test_consistency_issue_serialization, 3 parse tests)
- clippy clean (0 warnings)
- build clean
