use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use xz_knowledge_graph::KnowledgeGraph;
use xz_memory::MemorySystem;
use xz_provider::types::tool as provider;
use xz_provider::LlmProvider;

/// Context provided to tools during execution.
///
/// Contains references to shared infrastructure that a tool may need
/// to fulfill its function: the LLM provider for fallback calls, the
/// memory system for context storage, and an optional knowledge graph
/// for entity/relation queries.
#[derive(Debug, Clone)]
pub struct ToolContext {
    /// The novel or project identifier for this tool execution.
    pub novel_id: String,
    /// The chapter number being processed.
    pub chapter_number: u32,
    /// The LLM provider for making calls back to the model.
    pub provider: Arc<dyn LlmProvider>,
    /// The memory system for storing and retrieving context.
    pub memory: Arc<dyn MemorySystem>,
    /// Optional knowledge graph for entity/relation queries.
    pub knowledge_graph: Option<Arc<dyn KnowledgeGraph>>,
}

/// The output produced by executing a tool.
///
/// This is the xz-agent representation of a tool execution result. It can be
/// converted to/from [`xz_provider::types::tool::ToolResult`] for communication
/// with LLM providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    /// The text content of the tool's output.
    pub content: String,

    /// Optional structured data (e.g., JSON) returned by the tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured: Option<Value>,

    /// Whether the tool execution failed.
    #[serde(default)]
    pub is_error: bool,

    /// The ID of the tool call that produced this output, if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl From<ToolOutput> for provider::ToolResult {
    fn from(output: ToolOutput) -> Self {
        provider::ToolResult {
            tool_call_id: output.tool_call_id.unwrap_or_default(),
            content: output.content,
            is_error: output.is_error,
        }
    }
}

impl ToolOutput {
    /// Construct a [`ToolOutput`] from a provider-level [`ToolResult`].
    ///
    /// This is the reverse of `From<ToolOutput> for provider::ToolResult`.
    /// The `structured` field is set to `None` since provider results do not
    /// carry structured data separately.
    pub fn from_provider_result(result: provider::ToolResult) -> Self {
        ToolOutput {
            content: result.content,
            structured: None,
            is_error: result.is_error,
            tool_call_id: Some(result.tool_call_id),
        }
    }
}

/// Information about a tool call received from the LLM provider.
///
/// This is a simplified representation of [`xz_provider::types::tool::ToolCall`]
/// that preserves the essential fields for agent-side processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallExecInfo {
    /// The unique identifier of the tool call.
    pub id: String,
    /// The name of the tool being called.
    pub name: String,
    /// The arguments passed to the tool, as a JSON value.
    pub arguments: Value,
}

impl From<provider::ToolCall> for ToolCallExecInfo {
    fn from(call: provider::ToolCall) -> Self {
        ToolCallExecInfo {
            id: call.id,
            name: call.function_name,
            arguments: call.arguments,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Verify that converting a [`ToolOutput`] to a provider [`ToolResult`]
    /// correctly maps all fields.
    #[test]
    fn test_tool_output_to_provider_result() {
        let output = ToolOutput {
            content: "42".to_string(),
            structured: Some(json!({"answer": 42})),
            is_error: false,
            tool_call_id: Some("call_123".to_string()),
        };

        let result: provider::ToolResult = output.into();

        assert_eq!(result.tool_call_id, "call_123");
        assert_eq!(result.content, "42");
        assert!(!result.is_error);
    }

    /// Verify that converting a provider [`ToolCall`] to [`ToolCallExecInfo`]
    /// correctly maps all fields.
    #[test]
    fn test_tool_call_to_exec_info() {
        let call = provider::ToolCall {
            id: "call_456".to_string(),
            function_name: "get_weather".to_string(),
            arguments: json!({"city": "Beijing"}),
        };

        let info: ToolCallExecInfo = call.into();

        assert_eq!(info.id, "call_456");
        assert_eq!(info.name, "get_weather");
        assert_eq!(info.arguments, json!({"city": "Beijing"}));
    }

    /// Verify that [`ToolOutput::from_provider_result`] correctly maps all
    /// fields from a provider [`ToolResult`].
    #[test]
    fn test_tool_output_from_provider_result() {
        let result = provider::ToolResult {
            tool_call_id: "call_789".to_string(),
            content: "error: timeout".to_string(),
            is_error: true,
        };

        let output = ToolOutput::from_provider_result(result);

        assert_eq!(output.content, "error: timeout");
        assert!(output.is_error);
        assert_eq!(output.tool_call_id, Some("call_789".to_string()));
        assert!(output.structured.is_none());
    }

    /// Verify that a [`ToolOutput`] without a `tool_call_id` produces an empty
    /// string `tool_call_id` in the provider [`ToolResult`].
    #[test]
    fn test_tool_output_to_provider_result_no_id() {
        let output = ToolOutput {
            content: "done".to_string(),
            structured: None,
            is_error: false,
            tool_call_id: None,
        };

        let result: provider::ToolResult = output.into();

        assert_eq!(result.tool_call_id, "");
        assert_eq!(result.content, "done");
        assert!(!result.is_error);
    }
}
