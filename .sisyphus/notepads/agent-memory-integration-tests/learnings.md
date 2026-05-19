# Learnings: agent-memory-integration-tests

## Patterns
- `ProviderError::Internal` is a struct variant with named fields: `{ status: u16, message: String }`
- `xz_memory::Message` has no convenience constructors — use `Message::new(id, session_id, user_id, role, content, token_count)` with `uuid::Uuid::new_v4()` for IDs
- Integration test files in xz-agent/tests/ follow `use xz_agent::*;` pattern and use `#[tokio::test]`
- No file-level docstrings or section comments (matches existing test file style)
- `ToolContext` accepts `Option<Arc<dyn KnowledgeGraph>>` — pass `None` for memory-only tests
- `CharacterState` is a plain struct with all public fields — no builder pattern

## Mocking
- Minimal `MockLlmProvider` returning `ProviderError::Internal` is sufficient for tests that only need `Arc<dyn LlmProvider>` in ToolContext
- Tools implementing `AgentTool` need `async_trait` and must be `Send + Sync`
