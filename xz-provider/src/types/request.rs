use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::message::Message;
use super::tool::ToolDefinition;

/// 统一请求类型 —— 数据平面（发给 LLM 的内容）
/// 所有能力通过 option 字段开启，不需要多个方法
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// 目标模型。为 None 时由路由层根据 RouteContext 决定。
    pub model: Option<String>,
    pub messages: Vec<Message>,

    // ── 工具调用 ──
    /// 提供可用工具列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    /// 控制工具调用行为
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,

    // ── 结构化输出 ──
    /// 要求 LLM 按 JSON Schema 输出
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,

    // ── 生成参数 ──
    pub temperature: Option<f32>,
    pub max_tokens: Option<usize>,
    /// OpenAI o-series 使用 max_completion_tokens 而非 max_tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<usize>,
    pub top_p: Option<f32>,
    /// top_k 采样参数（Claude、Gemini 支持）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    pub stop: Option<Vec<String>>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    /// 随机种子，用于可复现输出（评测、调试场景必需）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    /// 推理努力程度（OpenAI o-series reasoning_effort / Claude thinking budget）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<ReasoningEffort>,
    /// 返回 log probabilities（调试、结构化分析场景）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<bool>,
    /// token 偏置（调整特定 token 的出现概率）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<HashMap<String, f32>>,

    // ── 流式选项 ──
    /// 流式模式下是否在末尾返回 usage
    pub stream_include_usage: Option<bool>,

    /// 请求唯一标识（用于 tracing，自动生成）
    #[serde(skip)]
    pub request_id: String,
}

impl CompletionRequest {
    pub fn new(model: impl Into<String>, messages: Vec<Message>) -> Self {
        Self {
            model: Some(model.into()),
            messages,
            tools: None,
            tool_choice: None,
            response_format: None,
            temperature: None,
            max_tokens: None,
            max_completion_tokens: None,
            top_p: None,
            top_k: None,
            stop: None,
            frequency_penalty: None,
            presence_penalty: None,
            seed: None,
            reasoning_effort: None,
            logprobs: None,
            logit_bias: None,
            stream_include_usage: None,
            request_id: uuid::Uuid::new_v4().to_string(),
        }
    }
}

impl Default for CompletionRequest {
    fn default() -> Self {
        Self {
            model: None,
            messages: Vec::new(),
            tools: None,
            tool_choice: None,
            response_format: None,
            temperature: None,
            max_tokens: None,
            max_completion_tokens: None,
            top_p: None,
            top_k: None,
            stop: None,
            frequency_penalty: None,
            presence_penalty: None,
            seed: None,
            reasoning_effort: None,
            logprobs: None,
            logit_bias: None,
            stream_include_usage: None,
            request_id: uuid::Uuid::new_v4().to_string(),
        }
    }
}

/// 控制工具调用行为
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolChoice {
    /// LLM 自主决定是否调用工具
    Auto,
    /// 必须调用工具
    Required,
    /// 禁止调用工具
    Disabled,
    /// 指定调用某个工具
    Specific { name: String },
}

/// 结构化输出格式要求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormat {
    /// 要求输出合法 JSON（不限定 schema）
    Json,
    /// 要求输出符合指定 JSON Schema（语法级保证）
    JsonSchema {
        schema: Value,
        name: String,
    },
}

/// 推理努力程度 — 控制 o-series / thinking 模型的推理深度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReasoningEffort {
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "high")]
    High,
}

/// 请求选项 —— 控制平面（告诉 Provider 怎么执行）
/// 与 CompletionRequest 分离，避免超时/取消等控制参数混入序列化数据
#[derive(Debug, Clone, Default)]
pub struct RequestOptions {
    /// 请求级超时。None 表示使用 Provider 默认超时。
    pub timeout: Option<Duration>,
    /// 取消令牌
    pub cancel: Option<crate::cancel::CancellationToken>,
    /// 请求级元数据（用于透传 trace_id 等）
    pub metadata: Option<HashMap<String, Value>>,
}

// ── Old StructuredRequest compat ──

/// (Deprecated) 结构化输出请求 —— 使用 CompletionRequest.response_format 代替
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub response_schema: Value,
    pub temperature: Option<f32>,
    pub max_tokens: Option<usize>,
    pub request_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completion_request_new() {
        let req = CompletionRequest::new("gpt-4", vec![Message::user("Hello")]);
        assert_eq!(req.model.as_deref(), Some("gpt-4"));
        assert_eq!(req.messages.len(), 1);
        assert!(!req.request_id.is_empty());
        assert!(req.tools.is_none());
        assert!(req.temperature.is_none());
        assert!(req.max_tokens.is_none());
        assert!(req.top_p.is_none());
        assert!(req.stop.is_none());
    }

    #[test]
    fn test_completion_request_new_empty_messages() {
        let req = CompletionRequest::new("gpt-4", vec![]);
        assert_eq!(req.model.as_deref(), Some("gpt-4"));
        assert!(req.messages.is_empty());
    }

    #[test]
    fn test_completion_request_default_model_none() {
        let req = CompletionRequest::default();
        assert!(req.model.is_none());
    }

    #[test]
    fn test_completion_request_unique_request_id() {
        let req1 = CompletionRequest::new("gpt-4", vec![]);
        let req2 = CompletionRequest::new("gpt-4", vec![]);
        assert_ne!(req1.request_id, req2.request_id);
    }

    #[test]
    fn test_request_options_default() {
        let opts = RequestOptions::default();
        assert!(opts.timeout.is_none());
        assert!(opts.cancel.is_none());
        assert!(opts.metadata.is_none());
    }

    #[test]
    fn test_tool_choice_serde() {
        let choices = vec![
            (ToolChoice::Auto, r#""auto""#),
            (ToolChoice::Required, r#""required""#),
            (ToolChoice::Disabled, r#""disabled""#),
        ];
        for (choice, expected) in choices {
            let json = serde_json::to_string(&choice).unwrap();
            assert_eq!(json, expected);
        }
    }

    #[test]
    fn test_tool_choice_specific_serde() {
        let choice = ToolChoice::Specific { name: "search".into() };
        let json = serde_json::to_string(&choice).unwrap();
        assert!(json.contains("search"));
    }

    #[test]
    fn test_response_format_json() {
        let fmt = ResponseFormat::Json;
        let json = serde_json::to_string(&fmt).unwrap();
        assert_eq!(json, r#""json""#);
    }

    #[test]
    fn test_response_format_json_schema() {
        let schema = serde_json::json!({"type": "object"});
        let fmt = ResponseFormat::JsonSchema {
            schema: schema.clone(),
            name: "MySchema".into(),
        };
        let json = serde_json::to_string(&fmt).unwrap();
        assert!(json.contains("MySchema"));
        assert!(json.contains("type"));
    }

    #[test]
    fn test_completion_request_serialize() {
        let req = CompletionRequest::new("gpt-4", vec![Message::user("Hi")]);
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("gpt-4"));
        assert!(json.contains("Hi"));
        // request_id is skipped in serialization
        assert!(!json.contains("request_id"));
    }

    #[test]
    fn test_reasoning_effort_serde() {
        assert_eq!(serde_json::to_string(&ReasoningEffort::Low).unwrap(), r#""low""#);
        assert_eq!(serde_json::to_string(&ReasoningEffort::Medium).unwrap(), r#""medium""#);
        assert_eq!(serde_json::to_string(&ReasoningEffort::High).unwrap(), r#""high""#);
    }

    #[test]
    fn test_completion_request_new_fields() {
        let mut req = CompletionRequest::new("gpt-4", vec![]);
        req.seed = Some(42);
        req.reasoning_effort = Some(ReasoningEffort::Medium);
        req.max_completion_tokens = Some(4000);
        req.top_k = Some(50);
        req.logprobs = Some(true);
        req.logit_bias = Some(HashMap::from([("hello".into(), 0.5)]));
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("42"));
        assert!(json.contains("medium"));
        assert!(json.contains("4000"));
        assert!(json.contains("50"));
    }
}
