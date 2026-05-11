use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::cache::CacheInfo;
use super::tool::ToolCall;

/// Token 用量
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    /// 缓存命中的 token 数（Prompt Caching）
    pub cached_tokens: Option<u32>,
}

impl TokenUsage {
    pub fn new(prompt_tokens: u32, completion_tokens: u32) -> Self {
        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
            cached_tokens: None,
        }
    }
}

/// 补全响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// 文本内容（可能为空，如 LLM 仅发起 tool call）
    pub content: Option<String>,
    /// 思考过程（Claude extended thinking / DeepSeek reasoning）
    pub thinking: Option<String>,
    /// LLM 发起的工具调用
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
    /// Token 用量
    pub usage: TokenUsage,
    /// 实际使用的模型名（可能与请求不同，如自动回退后）
    pub model: String,
    /// 结束原因
    pub finish_reason: FinishReason,
    /// 请求延迟（毫秒）
    pub latency_ms: u64,
    /// 缓存命中信息
    pub cache_info: Option<CacheInfo>,
}

/// 结束原因
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FinishReason {
    /// 正常结束
    #[serde(rename = "stop")]
    Stop,
    /// LLM 请求调用工具
    #[serde(rename = "tool_call")]
    ToolCall,
    /// 达到 token 上限
    #[serde(rename = "max_tokens")]
    MaxTokens,
    /// 内容过滤
    #[serde(rename = "content_filter")]
    ContentFilter,
}

/// 流式响应事件类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    /// 文本内容增量
    #[serde(rename = "content_delta")]
    ContentDelta {
        delta: String,
    },

    /// 工具调用增量 —— 函数名和参数片段逐步拼接
    #[serde(rename = "tool_call_delta")]
    ToolCallDelta {
        index: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        function_name: Option<String>,
        arguments_delta: String,
    },

    /// 思考过程增量（Claude extended thinking、DeepSeek reasoning）
    #[serde(rename = "thinking_delta")]
    ThinkingDelta {
        delta: String,
    },

    /// 图像增量（Gemini 图像生成等多模态输出场景）
    #[serde(rename = "image_delta")]
    ImageDelta {
        media_type: String,
        delta: String,
    },

    /// Token 用量更新
    #[serde(rename = "usage")]
    Usage {
        usage: TokenUsage,
    },

    /// 流结束信号
    #[serde(rename = "done")]
    Done {
        finish_reason: FinishReason,
        #[serde(skip_serializing_if = "Option::is_none")]
        usage: Option<TokenUsage>,
    },

    /// Provider 特有的事件（可扩展）
    #[serde(rename = "custom")]
    Custom {
        event: String,
        data: Value,
    },
}

// ── Old types compat ──

/// (Deprecated) 流式补全块 —— 使用 StreamEvent 代替
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub delta: String,
    pub finish_reason: Option<String>,
    pub usage: Option<TokenUsage>,
}

/// (Deprecated) 结构化输出响应 —— 直接使用 CompletionResponse
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredResponse<T> {
    pub parsed: T,
    pub raw: String,
    pub usage: TokenUsage,
    pub model: String,
    pub latency_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_usage_new() {
        let usage = TokenUsage::new(100, 50);
        assert_eq!(usage.prompt_tokens, 100);
        assert_eq!(usage.completion_tokens, 50);
        assert_eq!(usage.total_tokens, 150);
        assert!(usage.cached_tokens.is_none());
    }

    #[test]
    fn test_token_usage_new_zero() {
        let usage = TokenUsage::new(0, 0);
        assert_eq!(usage.total_tokens, 0);
        assert_eq!(usage.prompt_tokens, 0);
        assert_eq!(usage.completion_tokens, 0);
    }

    #[test]
    fn test_finish_reason_serde() {
        let cases = vec![
            (FinishReason::Stop, "\"stop\""),
            (FinishReason::ToolCall, "\"tool_call\""),
            (FinishReason::MaxTokens, "\"max_tokens\""),
            (FinishReason::ContentFilter, "\"content_filter\""),
        ];
        for (reason, expected) in cases {
            let json = serde_json::to_string(&reason).unwrap();
            assert_eq!(json, expected);
            let deserialized: FinishReason = serde_json::from_str(&json).unwrap();
            assert!(matches!(&deserialized, r if std::mem::discriminant(&reason) == std::mem::discriminant(r)));
        }
    }

    #[test]
    fn test_stream_event_content_delta() {
        let evt = StreamEvent::ContentDelta { delta: "Hello".into() };
        match evt {
            StreamEvent::ContentDelta { delta } => assert_eq!(delta, "Hello"),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_stream_event_tool_call_delta() {
        let evt = StreamEvent::ToolCallDelta {
            index: 0,
            id: Some("call_1".into()),
            function_name: Some("search".into()),
            arguments_delta: "{}".into(),
        };
        match evt {
            StreamEvent::ToolCallDelta { index, id, function_name, arguments_delta } => {
                assert_eq!(index, 0);
                assert_eq!(id.unwrap(), "call_1");
                assert_eq!(function_name.unwrap(), "search");
                assert_eq!(arguments_delta, "{}");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_stream_event_thinking_delta() {
        let evt = StreamEvent::ThinkingDelta { delta: "thinking...".into() };
        match evt {
            StreamEvent::ThinkingDelta { delta } => assert_eq!(delta, "thinking..."),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_stream_event_usage() {
        let usage = TokenUsage::new(10, 20);
        let evt = StreamEvent::Usage { usage: usage.clone() };
        match evt {
            StreamEvent::Usage { usage: u } => {
                assert_eq!(u.prompt_tokens, 10);
                assert_eq!(u.completion_tokens, 20);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_stream_event_done() {
        let evt = StreamEvent::Done {
            finish_reason: FinishReason::Stop,
            usage: None,
        };
        match evt {
            StreamEvent::Done { finish_reason, usage } => {
                assert!(matches!(finish_reason, FinishReason::Stop));
                assert!(usage.is_none());
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_stream_event_done_with_usage() {
        let usage = TokenUsage::new(5, 10);
        let evt = StreamEvent::Done {
            finish_reason: FinishReason::MaxTokens,
            usage: Some(usage),
        };
        match evt {
            StreamEvent::Done { finish_reason, usage } => {
                assert!(matches!(finish_reason, FinishReason::MaxTokens));
                assert_eq!(usage.unwrap().total_tokens, 15);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_stream_event_custom() {
        let evt = StreamEvent::Custom {
            event: "ping".into(),
            data: serde_json::json!({"key": "value"}),
        };
        match evt {
            StreamEvent::Custom { event, data } => {
                assert_eq!(event, "ping");
                assert_eq!(data["key"], "value");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_token_usage_serde() {
        let usage = TokenUsage::new(100, 50);
        let json = serde_json::to_string(&usage).unwrap();
        let deserialized: TokenUsage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.prompt_tokens, 100);
        assert_eq!(deserialized.completion_tokens, 50);
        assert_eq!(deserialized.total_tokens, 150);
    }

    #[test]
    fn test_completion_response_fields() {
        let usage = TokenUsage::new(10, 20);
        let resp = CompletionResponse {
            content: Some("Hello".into()),
            thinking: None,
            tool_calls: vec![],
            usage,
            model: "gpt-4".into(),
            finish_reason: FinishReason::Stop,
            latency_ms: 100,
            cache_info: None,
        };
        assert_eq!(resp.content.unwrap(), "Hello");
        assert_eq!(resp.model, "gpt-4");
        assert_eq!(resp.latency_ms, 100);
    }
}
