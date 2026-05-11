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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// 调用唯一标识，结果回传时需要
    pub id: String,
    pub function_name: String,
    /// 已解析的 JSON 参数
    pub arguments: Value,
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
