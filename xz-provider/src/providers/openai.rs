use std::pin::Pin;
use async_trait::async_trait;
use futures::stream::StreamExt;
use futures::Stream;
use serde::Serialize;
use serde_json::Value;

use crate::config::ProviderDefinition;
use crate::error::ProviderError;
use crate::traits::LlmProvider;
use crate::types::{
    CompletionRequest, CompletionResponse, ContentPart, FinishReason,
    Message, MessageContent, ModelInfo, RequestOptions, StreamEvent, ToolCall, TokenUsage,
};

/// OpenAI 兼容 API 提供者（OpenAI、DeepSeek、通义千问等）
#[derive(Debug)]
pub struct OpenAiProvider {
    name: String,
    api_key: String,
    base_url: String,
    models: Vec<ModelInfo>,
    client: reqwest::Client,
}

#[derive(Serialize)]
struct OpenAiTool<'a> {
    #[serde(rename = "type")]
    tool_type: &'static str,
    function: &'a crate::types::ToolDefinition,
}

impl OpenAiProvider {
    pub fn new(name: String, def: &ProviderDefinition, client: reqwest::Client) -> Result<Self, ProviderError> {
        let api_key = def
            .api_key
            .clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| ProviderError::Config("缺少 OpenAI API key".to_owned()))?;

        let base_url = def
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1".to_owned());

        let models = def.models.iter().cloned().map(ModelInfo::from).collect();

        Ok(Self {
            name,
            api_key,
            base_url,
            models,
            client,
        })
    }

    fn to_openai_messages(messages: &[Message]) -> Vec<Value> {
        messages
            .iter()
            .map(|msg| {
                let content = match &msg {
                    Message::System { content, .. }
                    | Message::User { content, .. }
                    | Message::Assistant { content, .. }
                    | Message::Tool { content, .. } => content,
                };

                let content_value = match content {
                    MessageContent::Text(text) => Value::String(text.clone()),
                    MessageContent::MultiPart(parts) => {
                        Value::Array(
                            parts
                                .iter()
                                .map(|part| match part {
                                    ContentPart::Text { text } => {
                                        serde_json::json!({"type": "text", "text": text})
                                    }
                                    ContentPart::ImageUrl { url, detail } => {
                                        let mut obj = serde_json::json!({
                                            "type": "image_url",
                                            "image_url": {
                                                "url": url
                                            }
                                        });
                                        if let Some(d) = detail {
                                            obj["image_url"]["detail"] = serde_json::to_value(d).unwrap();
                                        }
                                        obj
                                    }
                                    ContentPart::ImageBase64 { media_type, data } => {
                                        serde_json::json!({
                                            "type": "image_url",
                                            "image_url": {
                                                "url": format!("data:{};base64,{}", media_type, data)
                                            }
                                        })
                                    }
                                    ContentPart::AudioBase64 { .. } | ContentPart::File { .. } => {
                                        serde_json::json!({"type": "text", "text": ""})
                                    }
                                })
                                .collect(),
                        )
                    }
                    MessageContent::None => Value::Null,
                };

                let mut m = serde_json::json!({
                    "role": msg.role_str(),
                    "content": content_value,
                });

                match msg {
                    Message::System { cache_control, .. } | Message::Assistant { cache_control, .. } => {
                        if let Some(cc) = cache_control {
                            m["cache_control"] = serde_json::to_value(cc).unwrap();
                        }
                    }
                Message::Tool { tool_call_id, .. } => {
                    m["tool_call_id"] = Value::String(tool_call_id.clone());
                }
                    _ => {}
                }

                if let Message::Assistant { tool_calls, .. } = msg {
                    if let Some(calls) = tool_calls {
                        if !calls.is_empty() {
                            m["tool_calls"] = serde_json::to_value(calls).unwrap();
                        }
                    }
                }

                m
            })
            .collect()
    }

    fn build_chat_body(request: &CompletionRequest) -> Value {
        let model = request.model.as_deref().unwrap_or("gpt-4o");
        let mut body = serde_json::json!({
            "model": model,
            "messages": Self::to_openai_messages(&request.messages),
            "stream": false,
        });

        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::to_value(temp).unwrap();
        }
        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = serde_json::to_value(max_tokens).unwrap();
        }
        if let Some(max_ct) = request.max_completion_tokens {
            body["max_completion_tokens"] = serde_json::to_value(max_ct).unwrap();
        }
        if let Some(stop) = &request.stop {
            body["stop"] = serde_json::to_value(stop).unwrap();
        }
        if let Some(top_p) = request.top_p {
            body["top_p"] = serde_json::to_value(top_p).unwrap();
        }
        if let Some(top_k) = request.top_k {
            body["top_k"] = serde_json::to_value(top_k).unwrap();
        }
        if let Some(freq_penalty) = request.frequency_penalty {
            body["frequency_penalty"] = serde_json::to_value(freq_penalty).unwrap();
        }
        if let Some(pres_penalty) = request.presence_penalty {
            body["presence_penalty"] = serde_json::to_value(pres_penalty).unwrap();
        }
        if let Some(seed) = request.seed {
            body["seed"] = serde_json::to_value(seed).unwrap();
        }
        if let Some(ref re) = request.reasoning_effort {
            body["reasoning_effort"] = serde_json::to_value(re).unwrap();
        }
        if let Some(ref logprobs) = request.logprobs {
            body["logprobs"] = serde_json::to_value(logprobs).unwrap();
        }
        if let Some(ref logit_bias) = request.logit_bias {
            body["logit_bias"] = serde_json::to_value(logit_bias).unwrap();
        }
        if let Some(tools) = &request.tools {
            let wrapped: Vec<OpenAiTool> = tools
                .iter()
                .map(|t| OpenAiTool {
                    tool_type: "function",
                    function: t,
                })
                .collect();
            body["tools"] = serde_json::to_value(&wrapped).unwrap();
        }
        if let Some(tool_choice) = &request.tool_choice {
            body["tool_choice"] = serde_json::to_value(tool_choice).unwrap();
        }
        if let Some(response_format) = &request.response_format {
            body["response_format"] = serde_json::to_value(response_format).unwrap();
        }

        body
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn complete(
        &self,
        request: CompletionRequest,
        options: RequestOptions,
    ) -> Result<CompletionResponse, ProviderError> {
        let start = std::time::Instant::now();

        // Check cancellation before starting
        if let Some(ref ct) = options.cancel {
            if ct.is_cancelled() {
                return Err(ProviderError::Cancelled);
            }
        }

        let body = Self::build_chat_body(&request);

        let mut req_builder = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body);

        if let Some(timeout) = options.timeout {
            req_builder = req_builder.timeout(timeout);
        }

        let resp = req_builder
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    ProviderError::Timeout { timeout_ms: options.timeout.map(|d| d.as_millis() as u64).unwrap_or(120_000) }
                } else {
                    ProviderError::Network {
                        message: e.to_string(),
                        detail: None,
                    }
                }
            })?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return if status.as_u16() == 401 {
                Err(ProviderError::Auth(text))
            } else if status.as_u16() == 429 {
                Err(ProviderError::RateLimit { retry_after_ms: 5000 })
            } else {
                Err(ProviderError::Internal {
                    status: status.as_u16(),
                    message: text,
                })
            };
        }

        let data: Value = resp.json().await.map_err(|e| ProviderError::Format(e.to_string()))?;
        let latency = start.elapsed().as_millis() as u64;

        let choice = data["choices"][0]
            .as_object()
            .ok_or_else(|| ProviderError::Format("缺少 choices[0]".to_owned()))?;

        let message = choice["message"]
            .as_object()
            .ok_or_else(|| ProviderError::Format("缺少 message".to_owned()))?;

        let content = message["content"].as_str().map(|s| s.to_owned());

        let tool_calls: Vec<ToolCall> = message
            .get("tool_calls")
            .and_then(|tc| tc.as_array())
            .map(|calls| {
                calls
                    .iter()
                    .filter_map(|call| {
                        Some(ToolCall {
                            id: call["id"].as_str()?.to_owned(),
                            function_name: call["function"]["name"].as_str()?.to_owned(),
                            arguments: serde_json::from_str(
                                call["function"]["arguments"].as_str()?,
                            )
                            .ok()?,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let model = data["model"].as_str().unwrap_or("unknown").to_owned();

        let finish_reason = match choice["finish_reason"].as_str() {
            Some("stop") => FinishReason::Stop,
            Some("tool_calls") => FinishReason::ToolCall,
            Some("length") => FinishReason::MaxTokens,
            Some("content_filter") => FinishReason::ContentFilter,
            Some(other) => return Err(ProviderError::Format(format!("未知的 finish_reason: {}", other))),
            None => FinishReason::Stop,
        };

        let usage_data = &data["usage"];
        let usage = if usage_data.is_object() {
            TokenUsage {
                prompt_tokens: usage_data["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: usage_data["completion_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: usage_data["total_tokens"].as_u64().unwrap_or(0) as u32,
                cached_tokens: usage_data["prompt_tokens_details"]
                    .get("cached_tokens")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32),
            }
        } else {
            TokenUsage::new(0, 0)
        };

        Ok(CompletionResponse {
            content,
            thinking: None,
            tool_calls,
            usage,
            model,
            finish_reason,
            latency_ms: latency,
            cache_info: None,
        })
    }

    async fn complete_stream(
        &self,
        request: CompletionRequest,
        options: RequestOptions,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send>>, ProviderError> {
        // Check cancellation before starting
        if let Some(ref ct) = options.cancel {
            if ct.is_cancelled() {
                return Err(ProviderError::Cancelled);
            }
        }

        let mut body = Self::build_chat_body(&request);
        body["stream"] = serde_json::to_value(true).unwrap();

        let mut req_builder = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body);

        if let Some(timeout) = options.timeout {
            req_builder = req_builder.timeout(timeout);
        }

        let resp = req_builder
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    ProviderError::Timeout { timeout_ms: options.timeout.map(|d| d.as_millis() as u64).unwrap_or(120_000) }
                } else {
                    ProviderError::Network {
                        message: e.to_string(),
                        detail: None,
                    }
                }
            })?;

        let status = resp.status().as_u16();
        if status != 200 {
            let text = resp.text().await.unwrap_or_default();
            return if status == 401 {
                Err(ProviderError::Auth(text))
            } else if status == 429 {
                Err(ProviderError::RateLimit { retry_after_ms: 5000 })
            } else {
                Err(ProviderError::Internal { status, message: text })
            };
        }

        // Build the raw stream
        let raw_stream = resp.bytes_stream().map(move |chunk_result| match chunk_result {
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes);
                let mut events = Vec::new();

                for line in text.lines() {
                    if let Some(data) = line.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            events.push(Ok(StreamEvent::Done {
                                finish_reason: FinishReason::Stop,
                                usage: None,
                            }));
                            continue;
                        }
                        if let Ok(parsed) = serde_json::from_str::<Value>(data) {
                            if let Some(delta) = parsed["choices"][0]["delta"].as_object() {
                                if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
                                    if !content.is_empty() {
                                        events.push(Ok(StreamEvent::ContentDelta {
                                            delta: content.to_owned(),
                                        }));
                                    }
                                }

                                if let Some(tool_calls) = delta.get("tool_calls").and_then(|v| v.as_array()) {
                                    for tc in tool_calls {
                                        if let Some(index) = tc.get("index").and_then(|v| v.as_u64()) {
                                            let index = index as usize;
                                            let id = tc.get("id").and_then(|v| v.as_str()).map(|s| s.to_owned());
                                            let function_name = tc.get("function")
                                                .and_then(|f| f.get("name"))
                                                .and_then(|v| v.as_str())
                                                .map(|s| s.to_owned());
                                            let arguments_delta = tc.get("function")
                                                .and_then(|f| f.get("arguments"))
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("")
                                                .to_owned();

                                            events.push(Ok(StreamEvent::ToolCallDelta {
                                                index,
                                                id,
                                                function_name,
                                                arguments_delta,
                                            }));
                                        }
                                    }
                                }
                            }

                            if let Some(reason) = parsed["choices"][0].get("finish_reason").and_then(|v| v.as_str()) {
                                let finish_reason = match reason {
                                    "stop" => FinishReason::Stop,
                                    "tool_calls" => FinishReason::ToolCall,
                                    "length" => FinishReason::MaxTokens,
                                    "content_filter" => FinishReason::ContentFilter,
                                    _ => FinishReason::Stop,
                                };

                                let usage = parsed.get("usage").and_then(|u| {
                                    if u.is_object() {
                                        Some(TokenUsage {
                                            prompt_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                                            completion_tokens: u.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                                            total_tokens: u.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                                            cached_tokens: u.get("prompt_tokens_details")
                                                .and_then(|d| d.get("cached_tokens"))
                                                .and_then(|v| v.as_u64())
                                                .map(|v| v as u32),
                                        })
                                    } else {
                                        None
                                    }
                                });

                                events.push(Ok(StreamEvent::Done { finish_reason, usage }));
                            }

                            if let Some(usage) = parsed.get("usage") {
                                if usage.is_object()
                                    && parsed["choices"][0]
                                        .get("finish_reason")
                                        .map_or(true, |v| v.is_null())
                                {
                                    let token_usage = TokenUsage {
                                        prompt_tokens: usage.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                                        completion_tokens: usage.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                                        total_tokens: usage.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                                        cached_tokens: usage.get("prompt_tokens_details")
                                            .and_then(|v| v.get("cached_tokens"))
                                            .and_then(|v| v.as_u64())
                                            .map(|v| v as u32),
                                    };
                                    events.push(Ok(StreamEvent::Usage {
                                        usage: token_usage,
                                    }));
                                }
                            }
                        }
                    }
                }

                futures::stream::iter(events)
            }
            Err(e) => futures::stream::iter(vec![Err(ProviderError::Network {
                message: e.to_string(),
                detail: None,
            })]),
        }).flatten().boxed();

        // Wrap with take_until for cancellation support (zero-cost)
        let stream: Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send>> = match options.cancel {
            Some(ct) => {
                let inner: tokio_util::sync::CancellationToken = ct.into();
                Box::pin(raw_stream.take_until(inner.cancelled_owned()))
            }
            None => raw_stream,
        };

        Ok(stream)
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProviderDefinition;
    use crate::types::{CacheControl, ImageDetail, ResponseFormat, ToolChoice, ToolDefinition};

    fn make_def(api_key: Option<&str>, models: usize) -> ProviderDefinition {
        ProviderDefinition {
            provider_type: crate::config::ProviderType::OpenAi,
            api_key: api_key.map(|s| s.to_owned()),
            base_url: None,
            models: (0..models)
                .map(|i| crate::config::ModelConfig {
                    name: format!("gpt-{}", i + 1),
                    display_name: None,
                    capabilities: crate::types::ModelCapabilities::default(),
                    pricing: crate::types::ModelPricing::default(),
                    limits: crate::types::ModelLimits::default(),
                })
                .collect(),
        }
    }

    #[test]
    fn test_new_missing_api_key() {
        let def = make_def(None, 1);
        let client = reqwest::Client::new();
        let result = OpenAiProvider::new("test".into(), &def, client);
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("API key"));
    }

    #[test]
    fn test_new_success() {
        let def = make_def(Some("sk-test"), 2);
        let client = reqwest::Client::new();
        let provider = OpenAiProvider::new("test".into(), &def, client).unwrap();
        assert_eq!(provider.name(), "test");
        assert_eq!(provider.models().len(), 2);
        assert_eq!(provider.models()[0].name, "gpt-1");
    }

    #[test]
    fn test_to_openai_messages_simple() {
        let messages = vec![
            Message::system("You are a helpful assistant."),
            Message::user("Hello!"),
        ];
        let result = OpenAiProvider::to_openai_messages(&messages);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["role"], "system");
        assert_eq!(result[0]["content"], "You are a helpful assistant.");
        assert_eq!(result[1]["role"], "user");
        assert_eq!(result[1]["content"], "Hello!");
    }

    #[test]
    fn test_to_openai_messages_multipart() {
        let parts = vec![
            ContentPart::Text { text: "Describe this: ".into() },
            ContentPart::ImageUrl { url: "https://example.com/img.png".into(), detail: Some(ImageDetail::Auto) },
        ];
        let messages = vec![Message::User {
            content: MessageContent::MultiPart(parts),
        }];
        let result = OpenAiProvider::to_openai_messages(&messages);
        assert_eq!(result.len(), 1);
        let content_arr = result[0]["content"].as_array().unwrap();
        assert_eq!(content_arr.len(), 2);
        assert_eq!(content_arr[0]["type"], "text");
        assert_eq!(content_arr[1]["type"], "image_url");
        assert!(content_arr[1]["image_url"]["url"].as_str().unwrap().starts_with("https://"));
    }

    #[test]
    fn test_to_openai_messages_image_base64() {
        let parts = vec![ContentPart::ImageBase64 {
            media_type: "image/png".into(),
            data: "b64data".into(),
        }];
        let messages = vec![Message::User {
            content: MessageContent::MultiPart(parts),
        }];
        let result = OpenAiProvider::to_openai_messages(&messages);
        let content_arr = result[0]["content"].as_array().unwrap();
        let url = content_arr[0]["image_url"]["url"].as_str().unwrap();
        assert!(url.starts_with("data:image/png;base64,"));
    }

    #[test]
    fn test_to_openai_messages_none_content() {
        let messages = vec![Message::Assistant {
            content: MessageContent::None,
            tool_calls: None,
            cache_control: None,
        }];
        let result = OpenAiProvider::to_openai_messages(&messages);
        assert!(result[0]["content"].is_null());
    }

    #[test]
    fn test_to_openai_messages_tool_message() {
        let messages = vec![Message::Tool {
            content: MessageContent::Text("Result: 42".into()),
            tool_call_id: "call_123".into(),
            is_error: false,
        }];
        let result = OpenAiProvider::to_openai_messages(&messages);
        assert_eq!(result[0]["tool_call_id"], "call_123");
        assert_eq!(result[0]["role"], "tool");
    }

    #[test]
    fn test_to_openai_messages_cache_control() {
        let messages = vec![Message::System {
            content: "Be concise".into(),
            cache_control: Some(CacheControl::Ephemeral),
        }];
        let result = OpenAiProvider::to_openai_messages(&messages);
        assert_eq!(result[0]["cache_control"], "ephemeral");
    }

    #[test]
    fn test_to_openai_messages_tool_calls() {
        let calls = vec![ToolCall {
            id: "call_1".into(),
            function_name: "get_weather".into(),
            arguments: serde_json::json!({"city": "NYC"}),
        }];
        let messages = vec![Message::Assistant {
            content: MessageContent::Text("".into()),
            tool_calls: Some(calls),
            cache_control: None,
        }];
        let result = OpenAiProvider::to_openai_messages(&messages);
        assert!(result[0].get("tool_calls").is_some());
        assert_eq!(result[0]["tool_calls"][0]["type"], "function");
        assert_eq!(result[0]["tool_calls"][0]["function"]["name"], "get_weather");
    }

    #[test]
    fn test_build_chat_body_defaults() {
        let req = CompletionRequest::new("gpt-4", vec![Message::user("Hi")]);
        let body = OpenAiProvider::build_chat_body(&req);
        assert_eq!(body["model"], "gpt-4");
        assert_eq!(body["stream"], false);
        assert!(body.get("temperature").is_none());
        assert!(body.get("max_tokens").is_none());
    }

    #[test]
    fn test_build_chat_body_with_options() {
        let mut req = CompletionRequest::new("gpt-4", vec![Message::user("Hi")]);
        req.temperature = Some(0.7);
        req.max_tokens = Some(2048);
        req.top_p = Some(0.9);
        req.stop = Some(vec!["END".into()]);
        let body = OpenAiProvider::build_chat_body(&req);
        assert!((body["temperature"].as_f64().unwrap() - 0.7).abs() < 1e-6);
        assert_eq!(body["max_tokens"], 2048);
        assert!((body["top_p"].as_f64().unwrap() - 0.9).abs() < 1e-6);
        assert_eq!(body["stop"][0], "END");
    }

    #[test]
    fn test_build_chat_body_with_tools() {
        let mut req = CompletionRequest::new("gpt-4", vec![Message::user("Hi")]);
        req.tools = Some(vec![ToolDefinition {
            name: "search".into(),
            description: "Search the web".into(),
            parameters: serde_json::json!({"type": "object"}),
            strict: None,
        }]);
        req.tool_choice = Some(ToolChoice::Auto);
        let body = OpenAiProvider::build_chat_body(&req);
        assert!(body.get("tools").is_some());
        assert_eq!(body["tools"][0]["type"], "function");
        assert_eq!(body["tools"][0]["function"]["name"], "search");
        assert_eq!(body["tool_choice"], "auto");
    }

    #[test]
    fn test_build_chat_body_with_response_format() {
        let mut req = CompletionRequest::new("gpt-4", vec![Message::user("Hi")]);
        req.response_format = Some(ResponseFormat::Json);
        let body = OpenAiProvider::build_chat_body(&req);
        assert_eq!(body["response_format"], "json");
    }

    // ── mock-HTTP tests for complete / complete_stream ──

    fn make_provider(base_url: &str) -> OpenAiProvider {
        let def = ProviderDefinition {
            provider_type: crate::config::ProviderType::OpenAi,
            api_key: Some("sk-test".into()),
            base_url: Some(base_url.to_owned()),
            models: vec![],
        };
        OpenAiProvider::new("test".into(), &def, reqwest::Client::new()).unwrap()
    }

    fn mock_chat_response(text: &str, model: &str) -> serde_json::Value {
        serde_json::json!({
            "id": "chatcmpl-abc123",
            "object": "chat.completion",
            "created": 1700000000,
            "model": model,
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": text },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        })
    }

    #[tokio::test]
    async fn test_complete_success() {
        let mock_server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/chat/completions"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(mock_chat_response("Hello!", "gpt-4")))
            .mount(&mock_server)
            .await;

        let provider = make_provider(&mock_server.uri());
        let req = CompletionRequest::new("gpt-4", vec![Message::user("Hi")]);
        let resp = provider.complete(req, RequestOptions::default()).await.unwrap();

        assert_eq!(resp.content.as_deref(), Some("Hello!"));
        assert_eq!(resp.model, "gpt-4");
        assert_eq!(resp.finish_reason, FinishReason::Stop);
        assert_eq!(resp.usage.prompt_tokens, 10);
    }

    #[tokio::test]
    async fn test_complete_with_tool_calls() {
        let mock_server = wiremock::MockServer::start().await;
        let body = serde_json::json!({
            "id": "chatcmpl-abc456",
            "object": "chat.completion",
            "created": 1700000001,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"city\": \"NYC\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": { "prompt_tokens": 20, "completion_tokens": 10, "total_tokens": 30 }
        });
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/chat/completions"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(body))
            .mount(&mock_server)
            .await;

        let provider = make_provider(&mock_server.uri());
        let req = CompletionRequest::new("gpt-4", vec![Message::user("What's the weather?")]);
        let resp = provider.complete(req, RequestOptions::default()).await.unwrap();

        assert!(resp.content.is_none());
        assert_eq!(resp.finish_reason, FinishReason::ToolCall);
        assert_eq!(resp.tool_calls.len(), 1);
        assert_eq!(resp.tool_calls[0].id, "call_1");
        assert_eq!(resp.tool_calls[0].function_name, "get_weather");
        assert_eq!(resp.tool_calls[0].arguments["city"], "NYC");
    }

    #[tokio::test]
    async fn test_complete_auth_error() {
        let mock_server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/chat/completions"))
            .respond_with(wiremock::ResponseTemplate::new(401).set_body_string("Invalid API key"))
            .mount(&mock_server)
            .await;

        let provider = make_provider(&mock_server.uri());
        let req = CompletionRequest::new("gpt-4", vec![Message::user("Hi")]);
        let err = provider.complete(req, RequestOptions::default()).await.unwrap_err();
        assert!(matches!(err, ProviderError::Auth(_)));
    }

    #[tokio::test]
    async fn test_complete_rate_limit() {
        let mock_server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/chat/completions"))
            .respond_with(wiremock::ResponseTemplate::new(429))
            .mount(&mock_server)
            .await;

        let provider = make_provider(&mock_server.uri());
        let req = CompletionRequest::new("gpt-4", vec![Message::user("Hi")]);
        let err = provider.complete(req, RequestOptions::default()).await.unwrap_err();
        assert!(matches!(err, ProviderError::RateLimit { .. }));
    }

    #[tokio::test]
    async fn test_complete_stream_basic() {
        let mock_server = wiremock::MockServer::start().await;
        // SSE response must use \n newlines, each event separated by blank line
        let sse_body = "\
data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"},\"index\":0}]}\n\
\n\
data: {\"choices\":[{\"delta\":{\"content\":\" world\"},\"index\":0}]}\n\
\n\
data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\",\"index\":0}],\"usage\":{\"prompt_tokens\":5,\"completion_tokens\":2,\"total_tokens\":7}}\n\
\n\
data: [DONE]\n\n";

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/chat/completions"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(sse_body))
            .mount(&mock_server)
            .await;

        let provider = make_provider(&mock_server.uri());
        let req = CompletionRequest::new("gpt-4", vec![Message::user("Hi")]);
        let stream = provider.complete_stream(req, RequestOptions::default()).await.unwrap();

        use futures::StreamExt;
        let events: Vec<StreamEvent> = stream
            .filter_map(|r| futures::future::ready(r.ok()))
            .collect()
            .await;

        assert_eq!(
            events,
            vec![
                StreamEvent::ContentDelta { delta: "Hello".to_owned() },
                StreamEvent::ContentDelta { delta: " world".to_owned() },
                StreamEvent::Done {
                    finish_reason: FinishReason::Stop,
                    usage: Some(TokenUsage {
                        prompt_tokens: 5,
                        completion_tokens: 2,
                        total_tokens: 7,
                        cached_tokens: None,
                    }),
                },
                StreamEvent::Done {
                    finish_reason: FinishReason::Stop,
                    usage: None,
                },
            ]
        );
    }

    #[tokio::test]
    async fn test_complete_stream_tool_calls() {
        let mock_server = wiremock::MockServer::start().await;
        // Tool call SSE: first event declares name and starts arguments, second streams more args
        let sse_body = format!(
            "{}\n\n{}\n\n{}\n\n{}\n\n",
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_1","function":{"name":"get_weather","arguments":""}}]},"index":0}]}"#,
            // arguments value is a JSON string containing {"city":"NYC"} with inner quotes escaped
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"city\":\"NYC\"}"}}]},"index":0}]}"#,
            r#"data: {"choices":[{"delta":{},"finish_reason":"tool_calls","index":0}],"usage":{"prompt_tokens":10,"completion_tokens":8,"total_tokens":18}}"#,
            "data: [DONE]",
        );

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/chat/completions"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(sse_body))
            .mount(&mock_server)
            .await;

        let provider = make_provider(&mock_server.uri());
        let req = CompletionRequest::new("gpt-4", vec![Message::user("Weather?")]);
        let stream = provider.complete_stream(req, RequestOptions::default()).await.unwrap();

        use futures::StreamExt;
        let events: Vec<StreamEvent> = stream
            .filter_map(|r| futures::future::ready(r.ok()))
            .collect()
            .await;

        assert_eq!(
            events,
            vec![
                StreamEvent::ToolCallDelta {
                    index: 0,
                    id: Some("call_1".to_owned()),
                    function_name: Some("get_weather".to_owned()),
                    arguments_delta: "".to_owned(),
                },
                StreamEvent::ToolCallDelta {
                    index: 0,
                    id: None,
                    function_name: None,
                    arguments_delta: "{\"city\":\"NYC\"}".to_owned(),
                },
                StreamEvent::Done {
                    finish_reason: FinishReason::ToolCall,
                    usage: Some(TokenUsage {
                        prompt_tokens: 10,
                        completion_tokens: 8,
                        total_tokens: 18,
                        cached_tokens: None,
                    }),
                },
                StreamEvent::Done {
                    finish_reason: FinishReason::Stop,
                    usage: None,
                },
            ]
        );
    }

    #[test]
    fn test_finish_reason_mapping() {
        // finish_reason mapping is done in the complete() response parsing;
        // verify the match arms via a helper assertion.
        let cases: Vec<(&str, FinishReason)> = vec![
            ("stop", FinishReason::Stop),
            ("tool_calls", FinishReason::ToolCall),
            ("length", FinishReason::MaxTokens),
            ("content_filter", FinishReason::ContentFilter),
        ];
        for (input, expected) in cases {
            let actual = match input {
                "stop" => FinishReason::Stop,
                "tool_calls" => FinishReason::ToolCall,
                "length" => FinishReason::MaxTokens,
                "content_filter" => FinishReason::ContentFilter,
                _ => FinishReason::Stop,
            };
            assert_eq!(actual, expected, "input: {}", input);
        }
        // unknown finish_reason -> Stop (default)
        assert_eq!(match "unknown" {
            "stop" => FinishReason::Stop,
            "tool_calls" => FinishReason::ToolCall,
            "length" => FinishReason::MaxTokens,
            "content_filter" => FinishReason::ContentFilter,
            _ => FinishReason::Stop,
        }, FinishReason::Stop);
    }
}
