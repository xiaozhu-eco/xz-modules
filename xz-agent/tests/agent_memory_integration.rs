use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::Stream;
use serde_json::json;

use xz_agent::*;
use xz_memory::domain::character::{CharacterMemory, CharacterQuery, CharacterState};
use xz_memory::types::message::Role;
use xz_memory::{InMemoryMemory, MemorySystem};
use xz_provider::{
    CompletionRequest, CompletionResponse, LlmProvider, ModelInfo, ProviderError, RequestOptions,
    StreamEvent,
};

fn user_msg(session_id: &str, content: &str) -> xz_memory::Message {
    xz_memory::Message::new(
        uuid::Uuid::new_v4().to_string(),
        session_id.to_string(),
        "test-user".to_string(),
        Role::User,
        content.to_string(),
        0,
    )
}

#[derive(Debug)]
struct MockLlmProvider {
    model_name: String,
}

impl MockLlmProvider {
    fn new(model_name: &str) -> Self {
        Self {
            model_name: model_name.to_string(),
        }
    }
}

#[async_trait]
impl LlmProvider for MockLlmProvider {
    async fn complete(
        &self,
        _request: CompletionRequest,
        _options: RequestOptions,
    ) -> Result<CompletionResponse, ProviderError> {
        Err(ProviderError::Internal {
            status: 501,
            message: "mock provider: not implemented".to_string(),
        })
    }

    async fn complete_stream(
        &self,
        _request: CompletionRequest,
        _options: RequestOptions,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send>>,
        ProviderError,
    > {
        Err(ProviderError::Internal {
            status: 501,
            message: "mock provider: streaming not implemented".to_string(),
        })
    }

    fn models(&self) -> &[ModelInfo] {
        &[]
    }

    fn name(&self) -> &str {
        &self.model_name
    }
}

#[tokio::test]
async fn test_tool_context_with_memory() {
    let memory: Arc<dyn MemorySystem> = Arc::new(InMemoryMemory::new());
    let provider = Arc::new(MockLlmProvider::new("mock-model"));

    let context = ToolContext {
        novel_id: "test-novel".to_string(),
        chapter_number: 1,
        provider,
        memory: Arc::clone(&memory),
        knowledge_graph: None,
    };

    let msg = user_msg("session-1", "Hello, memory!");
    context
        .memory
        .append_message("session-1", msg)
        .await
        .unwrap();

    let recent = context
        .memory
        .get_recent_messages("session-1", 10)
        .await
        .unwrap();
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].content, "Hello, memory!");

    assert_eq!(context.novel_id, "test-novel");
    assert_eq!(context.chapter_number, 1);
    assert!(context.knowledge_graph.is_none());
}

struct MemoryAwareTool;

#[async_trait]
impl AgentTool for MemoryAwareTool {
    fn name(&self) -> &str {
        "memory_tool"
    }

    fn description(&self) -> &str {
        "A tool that reads/writes memory"
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "session_id": { "type": "string" },
                "message": { "type": "string" }
            },
            "required": ["session_id", "message"]
        })
    }

    async fn execute(
        &self,
        context: &ToolContext,
        args: serde_json::Value,
    ) -> Result<ToolOutput, AgentError> {
        let session_id = args["session_id"].as_str().unwrap_or("default-session");
        let message_text = args["message"].as_str().unwrap_or("no message");

        let msg = user_msg(session_id, message_text);
        context
            .memory
            .append_message(session_id, msg)
            .await
            .map_err(|e| AgentError::Io(format!("memory write failed: {}", e)))?;

        let recent = context
            .memory
            .get_recent_messages(session_id, 5)
            .await
            .map_err(|e| AgentError::Io(format!("memory read failed: {}", e)))?;

        let content = recent
            .last()
            .map(|m| m.content.clone())
            .unwrap_or_else(|| "no messages".to_string());

        Ok(ToolOutput {
            content: format!("stored and retrieved: {}", content),
            structured: None,
            is_error: false,
            tool_call_id: None,
        })
    }
}

#[tokio::test]
async fn test_tool_registry_with_memory_dependencies() {
    let memory: Arc<dyn MemorySystem> = Arc::new(InMemoryMemory::new());
    let provider = Arc::new(MockLlmProvider::new("mock-model"));

    let context = ToolContext {
        novel_id: "test-novel".to_string(),
        chapter_number: 2,
        provider,
        memory: Arc::clone(&memory),
        knowledge_graph: None,
    };

    let mut registry = ToolRegistry::new();
    registry.register(Box::new(MemoryAwareTool)).unwrap();

    let args = json!({
        "session_id": "test-session",
        "message": "integrated memory test"
    });
    let output = registry
        .execute("memory_tool", &context, args)
        .await
        .unwrap();

    assert!(!output.is_error);
    assert!(output.content.contains("integrated memory test"));
    assert!(output.content.contains("stored and retrieved"));

    let recent = memory
        .get_recent_messages("test-session", 10)
        .await
        .unwrap();
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].content, "integrated memory test");
}

#[tokio::test]
async fn test_tool_registry_unknown_tool() {
    let memory: Arc<dyn MemorySystem> = Arc::new(InMemoryMemory::new());
    let provider = Arc::new(MockLlmProvider::new("mock-model"));

    let context = ToolContext {
        novel_id: "test-novel".to_string(),
        chapter_number: 1,
        provider,
        memory,
        knowledge_graph: None,
    };

    let registry = ToolRegistry::new();

    let result = registry.execute("nonexistent_tool", &context, json!({})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_character_memory_via_memory_system() {
    let memory: Arc<dyn MemorySystem> = Arc::new(InMemoryMemory::new());
    let char_memory = CharacterMemory::new(Arc::clone(&memory), "novel-42");

    let hero = CharacterState {
        character_id: "char-1".to_string(),
        name: "Aria".to_string(),
        aliases: vec!["The Wanderer".to_string()],
        traits: vec![("brave".to_string(), "resolute".to_string())]
            .into_iter()
            .collect(),
        relationships: vec![("char-2".to_string(), "friend".to_string())]
            .into_iter()
            .collect(),
        last_appearance: None,
        appearance_count: 0,
        arc_status: "introduced".to_string(),
        voice_profile: Some("Confident, measured tone".to_string()),
        notes: "Protagonist with a mysterious past.".to_string(),
        updated_at: chrono::Utc::now(),
    };

    let villain = CharacterState {
        character_id: "char-2".to_string(),
        name: "Malachor".to_string(),
        aliases: vec![],
        traits: vec![("cunning".to_string(), "patient".to_string())]
            .into_iter()
            .collect(),
        relationships: vec![("char-1".to_string(), "nemesis".to_string())]
            .into_iter()
            .collect(),
        last_appearance: None,
        appearance_count: 0,
        arc_status: "dormant".to_string(),
        voice_profile: None,
        notes: "Ancient adversary.".to_string(),
        updated_at: chrono::Utc::now(),
    };

    char_memory.upsert_character(hero).await.unwrap();
    char_memory.upsert_character(villain).await.unwrap();

    let retrieved = char_memory.get_character("char-1").await.unwrap();
    assert!(retrieved.is_some());
    let aria = retrieved.unwrap();
    assert_eq!(aria.name, "Aria");
    assert_eq!(aria.arc_status, "introduced");
    assert!(aria.aliases.contains(&"The Wanderer".to_string()));
    assert_eq!(aria.traits.get("brave"), Some(&"resolute".to_string()));

    let all = char_memory.get_all_characters().await.unwrap();
    assert_eq!(all.len(), 2);

    let query = CharacterQuery {
        name_contains: Some("Aria".to_string()),
        ..Default::default()
    };
    let found = char_memory.get_relevant_characters(&query).await.unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].character_id, "char-1");

    char_memory
        .update_after_chapter(1, &["char-1".to_string()])
        .await
        .unwrap();

    let updated = char_memory.get_character("char-1").await.unwrap().unwrap();
    assert_eq!(updated.last_appearance, Some(1));
    assert_eq!(updated.appearance_count, 1);
    assert_eq!(updated.arc_status, "active");

    let dormant = char_memory.get_character("char-2").await.unwrap().unwrap();
    assert_eq!(dormant.arc_status, "dormant");

    char_memory.delete_character("char-2").await.unwrap();
    let after_delete = char_memory.get_character("char-2").await.unwrap();
    assert!(after_delete.is_none());
}

struct EchoTool;

#[async_trait]
impl AgentTool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> &str {
        "Returns the input message as-is"
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "message": { "type": "string" }
            },
            "required": ["message"]
        })
    }

    async fn execute(
        &self,
        _context: &ToolContext,
        args: serde_json::Value,
    ) -> Result<ToolOutput, AgentError> {
        let msg = args["message"].as_str().unwrap_or("no message");
        Ok(ToolOutput {
            content: format!("ECHO: {}", msg),
            structured: Some(json!({"echoed": msg})),
            is_error: false,
            tool_call_id: None,
        })
    }
}

#[tokio::test]
async fn test_tool_registry_roundtrip() {
    let memory: Arc<dyn MemorySystem> = Arc::new(InMemoryMemory::new());
    let provider = Arc::new(MockLlmProvider::new("mock-model"));
    let context = ToolContext {
        novel_id: "roundtrip-test".to_string(),
        chapter_number: 1,
        provider,
        memory,
        knowledge_graph: None,
    };

    let mut registry = ToolRegistry::new();
    registry.register(Box::new(EchoTool)).unwrap();

    assert!(registry.register(Box::new(EchoTool)).is_err());

    let definitions = registry.list_definitions();
    assert_eq!(definitions.len(), 1);
    assert_eq!(definitions[0].name, "echo");
    assert_eq!(definitions[0].description, "Returns the input message as-is");
    assert!(definitions[0].parameters.is_object());

    let output = registry
        .execute("echo", &context, json!({"message": "hello world"}))
        .await
        .unwrap();

    assert!(!output.is_error);
    assert_eq!(output.content, "ECHO: hello world");
    assert!(output.structured.is_some());
    assert_eq!(output.structured.unwrap()["echoed"], json!("hello world"));
}

#[test]
fn test_autonomous_config_with_memory() {
    let config = AutonomousConfig {
        model: "gpt-4o-mini".to_string(),
        max_tool_calls: 25,
        max_revision_rounds: 3,
        temperature: Some(0.8),
        max_tokens: Some(2048),
        novel_id: "novel-memory-test".to_string(),
        chapter_number: 7,
        fork_enabled: true,
        max_concurrent_forks: 5,
    };

    let json_str = serde_json::to_string(&config).unwrap();
    assert!(json_str.contains("novel-memory-test"));
    assert!(json_str.contains("gpt-4o-mini"));
    assert!(json_str.contains("25"));
    assert!(json_str.contains("2048"));

    let deserialized: AutonomousConfig = serde_json::from_str(&json_str).unwrap();
    assert_eq!(deserialized.model, "gpt-4o-mini");
    assert_eq!(deserialized.max_tool_calls, 25);
    assert_eq!(deserialized.max_revision_rounds, 3);
    assert_eq!(deserialized.temperature, Some(0.8));
    assert_eq!(deserialized.max_tokens, Some(2048));
    assert_eq!(deserialized.novel_id, "novel-memory-test");
    assert_eq!(deserialized.chapter_number, 7);
    assert!(deserialized.fork_enabled);
    assert_eq!(deserialized.max_concurrent_forks, 5);
}

#[test]
fn test_autonomous_config_default_serialization() {
    let config = AutonomousConfig::default();

    let json_str = serde_json::to_string(&config).unwrap();
    assert!(json_str.contains("default-novel"));
    assert!(json_str.contains("gpt-4o"));
    assert!(json_str.contains("50"));

    let deserialized: AutonomousConfig = serde_json::from_str(&json_str).unwrap();
    assert_eq!(deserialized.model, "gpt-4o");
    assert_eq!(deserialized.novel_id, "default-novel");
    assert_eq!(deserialized.chapter_number, 1);
    assert!(!deserialized.fork_enabled);
}
