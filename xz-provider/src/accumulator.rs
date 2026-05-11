use serde_json::Value;

use crate::error::ProviderError;
use crate::types::{StreamEvent, ToolCall};

/// 流式工具调用增量拼接器
///
/// OpenAI/Claude 的流式 tool calling 分步返回：
/// 1. 先返回 ToolCallDelta 携带 id、function_name
/// 2. 然后多次返回 arguments_delta JSON 片段
///
/// ToolCallAccumulator 将这些增量拼接为完整的 ToolCall。
#[derive(Debug, Default)]
pub struct ToolCallAccumulator {
    calls: Vec<PendingToolCall>,
}

/// 正在拼接中的工具调用
#[derive(Debug, Clone)]
pub struct PendingToolCall {
    pub id: String,
    pub function_name: String,
    pub arguments_json: String,
}

impl ToolCallAccumulator {
    pub fn new() -> Self {
        Self { calls: Vec::new() }
    }

    /// 处理流式事件，更新内部状态。
    /// 当新的工具调用开始时自动创建 PendingToolCall。
    pub fn process(&mut self, event: &StreamEvent) -> Option<&PendingToolCall> {
        match event {
            StreamEvent::ToolCallDelta {
                index,
                id,
                function_name,
                arguments_delta,
            } => {
                while self.calls.len() <= *index {
                    self.calls.push(PendingToolCall {
                        id: String::new(),
                        function_name: String::new(),
                        arguments_json: String::new(),
                    });
                }

                if let Some(id) = id {
                    self.calls[*index].id = id.clone();
                }
                if let Some(name) = function_name {
                    self.calls[*index].function_name = name.clone();
                }
                self.calls[*index].arguments_json.push_str(arguments_delta);

                Some(&self.calls[*index])
            }
            _ => None,
        }
    }

    /// 流结束后，将所有拼接结果解析为完整 ToolCall。
    /// 返回 Format 错误当 id 为空或 arguments JSON 解析失败。
    pub fn finalize(self) -> Result<Vec<ToolCall>, ProviderError> {
        self.calls
            .into_iter()
            .map(|pending| {
                if pending.id.is_empty() {
                    return Err(ProviderError::Format(
                        "ToolCallDelta completed with empty id".into(),
                    ));
                }
                if pending.function_name.is_empty() {
                    return Err(ProviderError::Format(format!(
                        "ToolCallDelta completed with empty function_name for id={}",
                        pending.id
                    )));
                }

                let arguments: Value = if pending.arguments_json.is_empty() {
                    Value::Object(Default::default())
                } else {
                    serde_json::from_str(&pending.arguments_json).map_err(|e| {
                        ProviderError::Format(format!(
                            "Failed to parse tool call arguments JSON for id={}: {}",
                            pending.id, e
                        ))
                    })?
                };

                Ok(ToolCall {
                    id: pending.id,
                    function_name: pending.function_name,
                    arguments,
                })
            })
            .collect()
    }

    /// 当前正在拼接中的调用数量
    pub fn pending_count(&self) -> usize {
        self.calls.len()
    }

    /// 清空所有待处理调用
    pub fn clear(&mut self) {
        self.calls.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::StreamEvent;

    #[test]
    fn test_single_tool_call() {
        let mut acc = ToolCallAccumulator::new();

        acc.process(&StreamEvent::ToolCallDelta {
            index: 0,
            id: Some("call_123".into()),
            function_name: Some("search".into()),
            arguments_delta: "".into(),
        });
        acc.process(&StreamEvent::ToolCallDelta {
            index: 0,
            id: None,
            function_name: None,
            arguments_delta: r#"{"query": "Rust 2024"}"#.into(),
        });

        let mut calls = acc.finalize().unwrap();
        assert_eq!(calls.len(), 1);
        let call = calls.remove(0);
        assert_eq!(call.id, "call_123");
        assert_eq!(call.function_name, "search");
        assert_eq!(call.arguments["query"], "Rust 2024");
    }

    #[test]
    fn test_multiple_tool_calls_parallel() {
        let mut acc = ToolCallAccumulator::new();

        acc.process(&StreamEvent::ToolCallDelta {
            index: 0,
            id: Some("call_0".into()),
            function_name: Some("search".into()),
            arguments_delta: r#"{"q": "a""#.into(),
        });
        acc.process(&StreamEvent::ToolCallDelta {
            index: 1,
            id: Some("call_1".into()),
            function_name: Some("code".into()),
            arguments_delta: r#"{"lang": "rust", "task": "setup"}"#.into(),
        });
        acc.process(&StreamEvent::ToolCallDelta {
            index: 0,
            id: None,
            function_name: None,
            arguments_delta: r#", "b": 2}"#.into(),
        });

        let mut calls = acc.finalize().unwrap();
        assert_eq!(calls.len(), 2);

        let call0 = calls.iter().find(|c| c.id == "call_0").unwrap();
        assert_eq!(call0.arguments["q"], "a");
        assert_eq!(call0.arguments["b"], 2);

        let call1 = calls.iter().find(|c| c.id == "call_1").unwrap();
        assert_eq!(call1.function_name, "code");
        assert_eq!(call1.arguments["lang"], "rust");
    }

    #[test]
    fn test_empty_arguments() {
        let mut acc = ToolCallAccumulator::new();

        acc.process(&StreamEvent::ToolCallDelta {
            index: 0,
            id: Some("call_empty".into()),
            function_name: Some("ping".into()),
            arguments_delta: "".into(),
        });

        let mut calls = acc.finalize().unwrap();
        assert_eq!(calls.len(), 1);
        let call = calls.remove(0);
        assert_eq!(call.arguments, Value::Object(Default::default()));
    }

    #[test]
    fn test_missing_id_error() {
        let mut acc = ToolCallAccumulator::new();
        acc.process(&StreamEvent::ToolCallDelta {
            index: 0,
            id: None,
            function_name: Some("search".into()),
            arguments_delta: "{}".into(),
        });

        let result = acc.finalize();
        assert!(result.is_err());
    }
}
