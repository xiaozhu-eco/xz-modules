use std::collections::HashMap;
use std::fmt;

use serde_json::Value;

use crate::error::AgentError;
use crate::tool::traits::AgentTool;
use crate::tool::types::{ToolContext, ToolOutput};
use xz_provider::ToolDefinition;

/// Central registry for all agent tools.
///
/// Stores tools by name and provides methods to look up, list,
/// and execute them. The registry is the bridge between the
/// LLM's function calling and the actual tool implementations.
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn AgentTool>>,
}

impl ToolRegistry {
    /// Creates a new, empty tool registry.
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Registers a tool in the registry.
    ///
    /// Returns an error if a tool with the same name is already
    /// registered. Tool names must be unique within a registry.
    pub fn register(&mut self, tool: Box<dyn AgentTool>) -> Result<(), AgentError> {
        let name = tool.name().to_string();
        if self.tools.contains_key(&name) {
            return Err(AgentError::Io(format!(
                "tool '{name}' is already registered"
            )));
        }
        self.tools.insert(name, tool);
        Ok(())
    }

    /// Looks up a tool by name, returning a reference if found.
    pub fn get(&self, name: &str) -> Option<&dyn AgentTool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    /// Converts all registered tools into `ToolDefinition` values
    /// suitable for sending to an LLM provider.
    ///
    /// Each tool's name, description, and parameter schema are
    /// packaged into the provider's tool definition format.
    pub fn list_definitions(&self) -> Vec<ToolDefinition> {
        self.tools
            .values()
            .map(|tool| ToolDefinition {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                parameters: tool.parameter_schema(),
                strict: None,
            })
            .collect()
    }

    /// Executes a tool by name with the given context and arguments.
    ///
    /// Returns `AgentError::NotFound` if no tool is registered under
    /// the given name. Otherwise, delegates to the tool's `execute`
    /// method.
    pub async fn execute(
        &self,
        name: &str,
        context: &ToolContext,
        args: Value,
    ) -> Result<ToolOutput, AgentError> {
        let tool = self
            .get(name)
            .ok_or_else(|| AgentError::NotFound(format!("tool '{name}' not found")))?;
        tool.execute(context, args).await
    }

    /// Returns the number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Returns `true` if the registry contains no tools.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ToolRegistry")
            .field("tool_count", &self.tools.len())
            .field(
                "tool_names",
                &self.tools.keys().collect::<Vec<&String>>(),
            )
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::json;

    /// Test tool that returns its name in the output content.
    struct TestTool {
        name: String,
        desc: String,
        schema: Value,
    }

    #[async_trait]
    impl AgentTool for TestTool {
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
            _ctx: &ToolContext,
            _args: Value,
        ) -> Result<ToolOutput, AgentError> {
            Ok(ToolOutput {
                content: format!("executed {}", self.name),
                structured: None,
                is_error: false,
                tool_call_id: None,
            })
        }
    }

    fn make_tool(name: &str) -> Box<TestTool> {
        Box::new(TestTool {
            name: name.to_string(),
            desc: format!("Tool {name}"),
            schema: json!({"type": "object", "properties": {}}),
        })
    }

    #[test]
    fn register_and_get() {
        let mut reg = ToolRegistry::new();
        reg.register(make_tool("tool_a")).unwrap();
        assert!(reg.get("tool_a").is_some());
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn duplicate_register_fails() {
        let mut reg = ToolRegistry::new();
        reg.register(make_tool("dup")).unwrap();
        let result = reg.register(make_tool("dup"));
        assert!(result.is_err());
    }

    #[test]
    fn list_definitions_returns_correct_count() {
        let mut reg = ToolRegistry::new();
        reg.register(make_tool("a")).unwrap();
        reg.register(make_tool("b")).unwrap();
        let defs = reg.list_definitions();
        assert_eq!(defs.len(), 2);
        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
    }

    #[test]
    fn len_and_is_empty() {
        let mut reg = ToolRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
        reg.register(make_tool("t")).unwrap();
        assert!(!reg.is_empty());
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn default_registry_is_empty() {
        let reg = ToolRegistry::default();
        assert!(reg.is_empty());
    }

    #[test]
    fn debug_format_includes_tool_count() {
        let mut reg = ToolRegistry::new();
        reg.register(make_tool("x")).unwrap();
        let debug_str = format!("{reg:?}");
        assert!(debug_str.contains("tool_count"));
        assert!(debug_str.contains("x"));
    }
}
