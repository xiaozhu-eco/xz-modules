use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 工具定义 —— 告诉 LLM "你可以调用哪些工具"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    /// JSON Schema 描述的参数结构
    pub parameters: Value,
    /// 强制 JSON 输出合规（OpenAI strict mode）
    /// Provider 实现层对不支持 strict 的 provider 静默忽略
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

/// LLM 返回的工具调用
#[derive(Debug, Clone, Deserialize)]
#[serde(from = "ToolCallWire")]
pub struct ToolCall {
    pub id: String,
    pub function_name: String,
    pub arguments: Value,
}

impl Serialize for ToolCall {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let wire = ToolCallWire {
            id: self.id.clone(),
            tool_type: "function".to_string(),
            function: ToolCallFunctionWire {
                name: self.function_name.clone(),
                arguments: serde_json::to_string(&self.arguments).unwrap_or_default(),
            },
        };
        wire.serialize(serializer)
    }
}

#[derive(Serialize, Deserialize)]
struct ToolCallWire {
    id: String,
    #[serde(rename = "type")]
    tool_type: String,
    function: ToolCallFunctionWire,
}

#[derive(Serialize, Deserialize)]
struct ToolCallFunctionWire {
    name: String,
    arguments: String,
}

impl From<ToolCallWire> for ToolCall {
    fn from(wire: ToolCallWire) -> Self {
        ToolCall {
            id: wire.id,
            function_name: wire.function.name,
            arguments: serde_json::from_str(&wire.function.arguments).unwrap_or(Value::Null),
        }
    }
}

/// 工具调用结果 —— 执行完工具后塞回 messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// 对应 ToolCall.id
    pub tool_call_id: String,
    /// 工具返回内容
    pub content: String,
    /// 工具执行是否失败
    #[serde(default)]
    pub is_error: bool,
}
