# Decisions

## Module placement
- Created `writer/src/workflow/agent.rs` (not a subdirectory like `workflow/agent/`) following the existing pattern where `workflow.rs` uses `#[path]` attributes to point to files in `workflow/`

## ChapterResult design
- Includes both `content` (chapter text) and `trajectory` (full execution log)
- Has `is_approved()` and `is_rejected()` convenience methods delegating to FinalVerdict
- `compute_word_count()` handles CJK (count chars) and ASCII (count whitespace-separated words) separately
- Derives Serialize/Deserialize for persistence

## Tool registration
- `build_tool_registry()` is a standalone public function (not a method on some struct) so other code can reuse it
- Returns `Result<ToolRegistry, anyhow::Error>` with `.context()` on each registration for clear error messages
- Uses `Box::new(ToolName::new())` for tools with new(), `Box::new(ToolName)` for unit-struct tools

## Task description
- Rich multi-step prompt with explicit numbered workflow steps
- References CHAPTER_COMPLETE marker for AutonomousLoop termination signal
- Configurable max revision rounds via format parameter
