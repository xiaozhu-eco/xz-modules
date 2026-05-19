use async_trait::async_trait;
use serde_json::Value;

use crate::error::AgentError;
use crate::tool::types::{ToolContext, ToolOutput};

/// A tool that can be invoked by the agent during LLM interactions.
///
/// Tools implement this trait to expose their functionality through
/// the tool registry, allowing the LLM to call them via function
/// calling. Each tool provides a name, description, JSON Schema
/// for its parameters, and an async execution method.
#[async_trait]
pub trait AgentTool: Send + Sync {
    /// Unique name for this tool, used as the function name in
    /// LLM function calling.
    fn name(&self) -> &str;

    /// Human-readable description of what this tool does,
    /// provided to the LLM to help it decide when to call the tool.
    fn description(&self) -> &str;

    /// Returns the JSON Schema describing the tool's parameters.
    /// This schema is sent to the LLM as part of the tool definition.
    fn parameter_schema(&self) -> Value;

    /// Execute the tool with the given context and arguments.
    ///
    /// The `context` provides access to shared infrastructure
    /// (provider, memory, knowledge graph). The `args` are the
    /// parsed JSON arguments from the LLM's tool call.
    async fn execute(
        &self,
        context: &ToolContext,
        args: Value,
    ) -> Result<ToolOutput, AgentError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A mock tool implementation used in tests.
    struct MockTool {
        name: String,
        desc: String,
        schema: Value,
    }

    #[async_trait]
    impl AgentTool for MockTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            &self.desc
        }

        fn parameter_schema(&self) -> Value {
            self.schema.clone()
        }

        async fn execute(
            &self,
            _context: &ToolContext,
            _args: Value,
        ) -> Result<ToolOutput, AgentError> {
            Ok(ToolOutput {
                content: "mock result".to_string(),
                structured: None,
                is_error: false,
                tool_call_id: None,
            })
        }
    }

    #[test]
    fn mock_tool_name_and_description() {
        let tool = MockTool {
            name: "mock_search".to_string(),
            desc: "A mock search tool".to_string(),
            schema: json!({"type": "object"}),
        };
        assert_eq!(tool.name(), "mock_search");
        assert_eq!(tool.description(), "A mock search tool");
    }

    #[test]
    fn mock_tool_parameter_schema_roundtrip() {
        let schema = json!({
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "Search query"}
            },
            "required": ["query"]
        });
        let tool = MockTool {
            name: "mock".to_string(),
            desc: "test".to_string(),
            schema: schema.clone(),
        };
        assert_eq!(tool.parameter_schema(), schema);
    }

    #[test]
    fn mock_tool_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        let tool = MockTool {
            name: "t".to_string(),
            desc: "d".to_string(),
            schema: json!({}),
        };
        assert_send_sync::<MockTool>();
        // Also verify via trait object reference
        let _: &dyn AgentTool = &tool;
    }
}
