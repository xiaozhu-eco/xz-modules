# T21: Implement three query tools

## Learned Patterns

- **AgentTool trait**: Methods are `name() -> &str`, `description() -> &str`, `parameter_schema() -> Value`, `execute(&self, context: &ToolContext, args: Value) -> Result<ToolOutput, AgentError>`.
- **ToolContext**: Provides `novel_id`, `chapter_number`, `provider`, `memory: Arc<dyn MemorySystem>`, `knowledge_graph: Option<Arc<dyn KnowledgeGraph>>`.
- **ToolOutput**: Has `content: String`, `structured: Option<Value>`, `is_error: bool`, `tool_call_id: Option<String>`.
- **JSON Schema**: Use `serde_json::json!` macro. Required fields listed in `"required": [...]` array.
- **Error handling**: Use `AgentError::StepFailed { step, reason }` for all errors. No unwrap/expect.
- **Two separate `PageRequest` types**: `xz_memory::PageRequest` (for fact recall) and `xz_knowledge_graph::PageRequest` (for entity query). They are distinct types.

## Key API Details

- `xz_memory::domain::character::CharacterMemory::new(memory, novel_id)` then `.get_character(id)` returns `Result<Option<CharacterState>, MemoryError>`.
- `CharacterState` has `character_id`, `name`, `aliases`, `traits: HashMap<String,String>`, `relationships: HashMap<String,String>`, `voice_profile: Option<String>`, `arc_status`, etc.
- `xz_knowledge_graph::EntityQuery` uses `page: PageRequest` (not direct `limit` field).
- `xz_knowledge_graph::Entity.attributes: HashMap<String, AttributeValue>` where `AttributeValue` has `.value: String`.
- `xz_knowledge_graph::KnowledgeGraph::get_neighbors(id, depth)` returns `SubGraph { center, entities, relations }`.
