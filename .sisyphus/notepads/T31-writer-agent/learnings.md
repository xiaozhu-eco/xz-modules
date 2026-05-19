# T31: Writer bin integration with AutonomousLoop

## Key Patterns

### FinalVerdict has struct variants
- `Approved` (unit variant)
- `ApprovedWithWarnings { warnings: Vec<SafetyViolation> }`
- `Rejected { violations: Vec<SafetyViolation> }`
- Has `is_approved()` method that matches both Approved and ApprovedWithWarnings{..}

### Tool constructors
- 15 tools have explicit `new()` returning `Self`
- 10 tools are unit structs with no `new()` — instantiated as `QueryArcProgressTool` directly
- Tool categories: 9 query + 6 creation + 6 review + 4 management = 25 total

### AutonomousLoop API
- `AutonomousLoop::new(config: AutonomousConfig, tool_registry: Arc<ToolRegistry>)`
- `.run(task_description, provider, memory, knowledge_graph) -> AutonomousResult`
- `AutonomousResult` has `chapter_content`, `trajectory`, `safety_report`, `total_tool_calls`, `total_steps`, `duration_ms`, `final_verdict`

### Dependency chain
- writer crate needs direct deps on xz-provider, xz-memory, xz-knowledge-graph (not just transitive via awe-tools)
- awe-tools already depends on all xz-modules crates

### Pre-existing build issues
- writer crate has 19 pre-existing errors in `post_processing.rs` (volumn_outline module missing)
- These errors block `cargo test` but not `cargo check` for individual modules
- Agent module compiles cleanly independent of these issues
