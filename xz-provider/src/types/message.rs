use serde::{Deserialize, Serialize};
use std::fmt;

use super::cache::CacheControl;
use super::tool::ToolCall;

// ── Message ──

/// 消息 —— 对话中的单条消息
///
/// 使用 enum 而非 flat struct，让非法状态不可表示：
/// - 只有 Assistant 可以携带 tool_calls
/// - 只有 Tool 必须携带 tool_call_id
/// - 编译时就能发现角色与字段的错误组合
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role")]
pub enum Message {
    #[serde(rename = "system")]
    System {
        content: MessageContent,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },

    #[serde(rename = "user")]
    User {
        content: MessageContent,
    },

    #[serde(rename = "assistant")]
    Assistant {
        /// 文本内容（可能为空，如 LLM 仅发起 tool call）
        content: MessageContent,
        /// LLM 发起的工具调用
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_calls: Option<Vec<ToolCall>>,
        /// 缓存控制标记
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },

    #[serde(rename = "tool")]
    Tool {
        /// 工具执行结果
        content: MessageContent,
        /// 对应 ToolCall.id，必填
        tool_call_id: String,
        /// 工具执行是否出错
        #[serde(default)]
        is_error: bool,
    },
}

impl Message {
    pub fn system(text: &str) -> Self {
        Message::System {
            content: MessageContent::Text(text.into()),
            cache_control: None,
        }
    }

    pub fn user(text: &str) -> Self {
        Message::User {
            content: MessageContent::Text(text.into()),
        }
    }

    pub fn assistant(text: &str) -> Self {
        Message::Assistant {
            content: MessageContent::Text(text.into()),
            tool_calls: None,
            cache_control: None,
        }
    }

    pub fn tool_result(tool_call_id: &str, content: &str) -> Self {
        Message::Tool {
            content: MessageContent::Text(content.into()),
            tool_call_id: tool_call_id.into(),
            is_error: false,
        }
    }

    pub fn tool_error(tool_call_id: &str, error: &str) -> Self {
        Message::Tool {
            content: MessageContent::Text(error.into()),
            tool_call_id: tool_call_id.into(),
            is_error: true,
        }
    }

    /// 返回消息的角色标识字符串
    pub fn role_str(&self) -> &'static str {
        match self {
            Message::System { .. } => "system",
            Message::User { .. } => "user",
            Message::Assistant { .. } => "assistant",
            Message::Tool { .. } => "tool",
        }
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Message::System { content, .. }
            | Message::User { content, .. }
            | Message::Assistant { content, .. }
            | Message::Tool { content, .. } => match content {
                MessageContent::Text(t) => write!(f, "{}", t),
                MessageContent::MultiPart(parts) => {
                    for part in parts {
                        match part {
                            ContentPart::Text { text, .. } => write!(f, "{}", text)?,
                            ContentPart::ImageUrl { .. } => write!(f, "[Image]")?,
                            ContentPart::ImageBase64 { .. } => write!(f, "[Image(base64)]")?,
                            ContentPart::AudioBase64 { .. } => write!(f, "[Audio]")?,
                            ContentPart::File { filename, .. } => {
                                if let Some(name) = filename {
                                    write!(f, "[File: {}]", name)?
                                } else {
                                    write!(f, "[File]")?
                                }
                            }
                        }
                    }
                    Ok(())
                }
                MessageContent::None => Ok(()),
            },
        }
    }
}

// ── MessageContent ──

/// 消息内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent {
    /// 纯文本
    Text(String),
    /// 多模态内容（文本 + 图片 + 文件）
    MultiPart(Vec<ContentPart>),
    /// 空内容（Assistant 仅发起 tool call 时）
    None,
}

impl From<String> for MessageContent {
    fn from(s: String) -> Self {
        MessageContent::Text(s)
    }
}

impl From<&str> for MessageContent {
    fn from(s: &str) -> Self {
        MessageContent::Text(s.to_owned())
    }
}

impl From<Vec<ContentPart>> for MessageContent {
    fn from(parts: Vec<ContentPart>) -> Self {
        MessageContent::MultiPart(parts)
    }
}

// ── ContentPart ──

/// 多模态内容片段
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentPart {
    #[serde(rename = "text")]
    Text { text: String },

    #[serde(rename = "image_url")]
    ImageUrl {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<ImageDetail>,
    },

    #[serde(rename = "image_base64")]
    ImageBase64 {
        media_type: String,
        data: String,
    },

    /// 音频输入（GPT-4o-audio、Gemini Audio 等多模态音频模型）
    #[serde(rename = "audio_base64")]
    AudioBase64 {
        media_type: String,
        data: String,
    },

    /// 文件/文档引用 — 轻量引用，仅携带 file_id
    /// 文件上传/管理由独立 crate（如 xz-upload）负责，不混入 provider 层
    #[serde(rename = "file")]
    File {
        file_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        filename: Option<String>,
    },
}

// ── ImageDetail ──

/// 图片细节级别
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImageDetail {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "high")]
    High,
}

// ── Deprecated/Compat ──

/// 兼容旧的 Role 枚举（v1 代码过渡使用）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_message() {
        let msg = Message::system("You are a helpful assistant.");
        assert_eq!(msg.role_str(), "system");
        match msg {
            Message::System { content, cache_control } => {
                assert!(matches!(content, MessageContent::Text(_)));
                assert!(cache_control.is_none());
            }
            _ => panic!("Expected System variant"),
        }
    }

    #[test]
    fn test_user_message() {
        let msg = Message::user("Hello!");
        assert_eq!(msg.role_str(), "user");
        match msg {
            Message::User { content } => {
                assert!(matches!(content, MessageContent::Text(_)));
            }
            _ => panic!("Expected User variant"),
        }
    }

    #[test]
    fn test_assistant_message() {
        let msg = Message::assistant("Hi!");
        assert_eq!(msg.role_str(), "assistant");
        match msg {
            Message::Assistant { content, tool_calls, cache_control } => {
                assert!(matches!(content, MessageContent::Text(_)));
                assert!(tool_calls.is_none());
                assert!(cache_control.is_none());
            }
            _ => panic!("Expected Assistant variant"),
        }
    }

    #[test]
    fn test_tool_result() {
        let msg = Message::tool_result("call_1", "result data");
        assert_eq!(msg.role_str(), "tool");
        match msg {
            Message::Tool { content, tool_call_id, is_error } => {
                assert!(matches!(content, MessageContent::Text(_)));
                assert_eq!(tool_call_id, "call_1");
                assert!(!is_error);
            }
            _ => panic!("Expected Tool variant"),
        }
    }

    #[test]
    fn test_tool_error() {
        let msg = Message::tool_error("call_1", "error!");
        match msg {
            Message::Tool { ref tool_call_id, is_error, .. } => {
                assert!(is_error);
                assert_eq!(tool_call_id, "call_1");
            }
            _ => panic!("Expected Tool variant"),
        }
    }

    #[test]
    fn test_role_str_all_variants() {
        assert_eq!(Message::system("a").role_str(), "system");
        assert_eq!(Message::user("a").role_str(), "user");
        assert_eq!(Message::assistant("a").role_str(), "assistant");
        assert_eq!(Message::tool_result("1", "a").role_str(), "tool");
    }

    #[test]
    fn test_message_content_from_string() {
        let content: MessageContent = "hello".to_string().into();
        assert!(matches!(content, MessageContent::Text(_)));
    }

    #[test]
    fn test_message_content_from_str() {
        let content: MessageContent = "hello".into();
        assert!(matches!(content, MessageContent::Text(_)));
    }

    #[test]
    fn test_message_content_from_vec_parts() {
        let parts = vec![ContentPart::Text { text: "hello".into() }];
        let content: MessageContent = parts.into();
        assert!(matches!(content, MessageContent::MultiPart(_)));
    }

    #[test]
    fn test_message_display_text() {
        let msg = Message::user("Hello!");
        assert_eq!(format!("{}", msg), "Hello!");
    }

    #[test]
    fn test_message_display_multipart() {
        let parts = vec![
            ContentPart::Text { text: "Check ".into() },
            ContentPart::ImageUrl { url: "https://example.com/img.png".into(), detail: None },
        ];
        let content = MessageContent::MultiPart(parts);
        let msg = Message::User { content };
        assert_eq!(format!("{}", msg), "Check [Image]");
    }

    #[test]
    fn test_message_display_none() {
        let msg = Message::Assistant {
            content: MessageContent::None,
            tool_calls: None,
            cache_control: None,
        };
        assert_eq!(format!("{}", msg), "");
    }

    #[test]
    fn test_content_part_image_base64_display() {
        let part = ContentPart::ImageBase64 {
            media_type: "image/png".into(),
            data: "base64data".into(),
        };
        let msg = Message::User {
            content: MessageContent::MultiPart(vec![part]),
        };
        assert_eq!(format!("{}", msg), "[Image(base64)]");
    }

    #[test]
    fn test_message_serde_roundtrip() {
        let msg = Message::assistant("Hello");
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(msg.role_str(), deserialized.role_str());
    }

    #[test]
    fn test_image_detail_serde() {
        let json = r#""auto""#;
        let detail: ImageDetail = serde_json::from_str(json).unwrap();
        assert!(matches!(detail, ImageDetail::Auto));
    }

    #[test]
    fn test_content_part_serde() {
        let part = ContentPart::Text { text: "hello".into() };
        let json = serde_json::to_string(&part).unwrap();
        let deserialized: ContentPart = serde_json::from_str(&json).unwrap();
        match deserialized {
            ContentPart::Text { text } => assert_eq!(text, "hello"),
            _ => panic!("Expected Text variant"),
        }
    }
}
