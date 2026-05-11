use std::pin::Pin;
use std::time::Instant;

use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use serde_json::Value;

use crate::config::ProviderDefinition;
use crate::error::ProviderError;
use crate::traits::LlmProvider;
use crate::types::{
    CacheControl, CacheInfo, CompletionRequest, CompletionResponse, ContentPart, FinishReason,
    Message, MessageContent, ModelInfo, RequestOptions, StreamEvent, TokenUsage, ToolCall,
};

/// Anthropic Claude API 提供者
#[derive(Debug)]
pub struct ClaudeProvider {
    name: String,
    api_key: String,
    base_url: String,
    anthropic_version: String,
    models: Vec<ModelInfo>,
    client: reqwest::Client,
}

impl ClaudeProvider {
    pub fn new(name: String, def: &ProviderDefinition, client: reqwest::Client) -> Result<Self, ProviderError> {
        let api_key = def
            .api_key
            .clone()
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
            .ok_or_else(|| ProviderError::Config("缺少 Anthropic API key".to_owned()))?;

        let base_url = def
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.anthropic.com/v1".to_owned());

        let models = def.models.iter().map(|m| {
            let mut info = ModelInfo::from(m.clone());
            info.provider = Some(name.clone());
            info
        }).collect();

        Ok(Self {
            name,
            api_key,
            base_url,
            anthropic_version: "2023-06-01".to_owned(),
            models,
            client,
        })
    }

    fn convert_messages(messages: &[Message]) -> (Vec<Value>, Option<String>, Vec<Value>) {
        let mut system_blocks = Vec::new();
        let mut msgs = Vec::new();
        let mut system_text = None;

        for msg in messages {
            match msg {
                Message::System { content, cache_control } => {
                    if let MessageContent::Text(text) = content {
                        if let Some(cc) = cache_control {
                            system_blocks.push(serde_json::json!({
                                "type": "text",
                                "text": text,
                                "cache_control": {"type": "ephemeral"}
                            }));
                        } else {
                            system_text = Some(text.clone());
                        }
                    } else if let MessageContent::MultiPart(parts) = content {
                        for part in parts {
                            let block = match part {
                                ContentPart::Text { text } => {
                                    let mut block = serde_json::json!({
                                        "type": "text",
                                        "text": text,
                                    });
                                    if let Some(cc) = cache_control {
                                        block["cache_control"] = serde_json::json!({"type": "ephemeral"});
                                    }
                                    block
                                }
                                ContentPart::ImageBase64 { media_type, data } => {
                                    serde_json::json!({
                                        "type": "image",
                                        "source": {
                                            "type": "base64",
                                            "media_type": media_type,
                                            "data": data,
                                        }
                                    })
                                }
                                _ => continue,
                            };
                            system_blocks.push(block);
                        }
                    }
                }
                Message::User { content } => {
                    let content_blocks = Self::convert_content(content, None);
                    msgs.push(serde_json::json!({
                        "role": "user",
                        "content": content_blocks,
                    }));
                }
                Message::Assistant { content, tool_calls, cache_control } => {
                    let mut content_blocks = Vec::new();

                    if let Some(calls) = tool_calls {
                        for call in calls {
                            content_blocks.push(serde_json::json!({
                                "type": "tool_use",
                                "id": call.id,
                                "name": call.function_name,
                                "input": call.arguments,
                            }));
                        }
                    }

                    if !matches!(content, MessageContent::None) {
                        let text_blocks = Self::convert_content(content, cache_control.as_ref());
                        content_blocks.extend(text_blocks);
                    }

                    msgs.push(serde_json::json!({
                        "role": "assistant",
                        "content": content_blocks,
                    }));
                }
                Message::Tool { content, tool_call_id, is_error } => {
                    let text = match content {
                        MessageContent::Text(t) => t.clone(),
                        _ => String::new(),
                    };
                    msgs.push(serde_json::json!({
                        "role": "user",
                        "content": [{
                            "type": "tool_result",
                            "tool_use_id": tool_call_id,
                            "content": text,
                            "is_error": is_error,
                        }],
                    }));
                }
            }
        }

        let system = if system_blocks.is_empty() {
            system_text
        } else if system_blocks.len() == 1 {
            system_blocks[0]["text"].as_str().map(|s| s.to_owned())
        } else {
            None
        };

        let system_content = if system_blocks.len() > 1 {
            Some(system_blocks)
        } else {
            None
        };

        (msgs, system, system_content.unwrap_or_default())
    }

    fn convert_content(content: &MessageContent, cache_control: Option<&CacheControl>) -> Vec<Value> {
        match content {
            MessageContent::Text(text) => {
                let mut block = serde_json::json!({
                    "type": "text",
                    "text": text,
                });
                if let Some(cc) = cache_control {
                    block["cache_control"] = serde_json::json!({"type": "ephemeral"});
                }
                vec![block]
            }
            MessageContent::MultiPart(parts) => {
                parts
                    .iter()
                    .map(|part| match part {
                        ContentPart::Text { text } => {
                            serde_json::json!({"type": "text", "text": text})
                        }
                        ContentPart::ImageUrl { url, detail } => {
                            serde_json::json!({
                                "type": "image",
                                "source": {
                                    "type": "url",
                                    "url": url,
                                }
                            })
                        }
                        ContentPart::ImageBase64 { media_type, data } => {
                            serde_json::json!({
                                "type": "image",
                                "source": {
                                    "type": "base64",
                                    "media_type": media_type,
                                    "data": data,
                                }
                            })
                        }
                    })
                    .collect()
            }
            MessageContent::None => Vec::new(),
        }
    }

    fn parse_finish_reason(reason: &str) -> FinishReason {
        match reason {
            "end_turn" => FinishReason::Stop,
            "tool_use" => FinishReason::ToolCall,
            "max_tokens" => FinishReason::MaxTokens,
            "stop_sequence" => FinishReason::Stop,
            _ => FinishReason::Stop,
        }
    }
}

#[async_trait]
impl LlmProvider for ClaudeProvider {
    async fn complete(
        &self,
        request: CompletionRequest,
        options: RequestOptions,
    ) -> Result<CompletionResponse, ProviderError> {
        let start = Instant::now();

        let (messages, system, system_blocks) = Self::convert_messages(&request.messages);

        let mut body = serde_json::json!({
            "model": request.model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(4096),
        });

        if let Some(temp) = request.temperature {
            body["temperature"] = Value::from(temp);
        }
        if let Some(top_p) = request.top_p {
            body["top_p"] = Value::from(top_p);
        }
        if let Some(s) = system {
            body["system"] = Value::String(s);
        } else if !system_blocks.is_empty() {
            body["system"] = Value::Array(system_blocks);
        }
        if let Some(stop) = request.stop {
            body["stop_sequences"] = Value::Array(stop.into_iter().map(Value::String).collect());
        }

        if let Some(tools) = &request.tools {
            let tool_defs: Vec<Value> = tools.iter().map(|t| {
                serde_json::json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.parameters,
                })
            }).collect();
            body["tools"] = Value::Array(tool_defs);
        }

        if let Some(tc) = &request.tool_choice {
            body["tool_choice"] = match tc {
                crate::types::ToolChoice::Auto => "auto".into(),
                crate::types::ToolChoice::Required => "any".into(),
                crate::types::ToolChoice::Disabled => "auto".into(),
                crate::types::ToolChoice::Specific { name } => {
                    serde_json::json!({"type": "tool", "name": name})
                }
            };
        }

        let mut req_builder = self
            .client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", &self.anthropic_version)
            .header("content-type", "application/json");

        if let Some(timeout) = options.timeout {
            req_builder = req_builder.timeout(timeout);
        }

        let resp = req_builder
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Network {
                message: e.to_string(),
                detail: Some(format!("{:?}", e.kind())),
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

        let content_blocks = data["content"].as_array().unwrap_or(&vec![]);
        let mut text_content = String::new();
        let mut tool_calls = Vec::new();

        for block in content_blocks {
            if let Some(block_type) = block["type"].as_str() {
                match block_type {
                    "text" => {
                        if let Some(text) = block["text"].as_str() {
                            text_content.push_str(text);
                        }
                    }
                    "tool_use" => {
                        let id = block["id"].as_str().unwrap_or("").to_owned();
                        let name = block["name"].as_str().unwrap_or("").to_owned();
                        let input = block["input"].clone();
                        tool_calls.push(ToolCall {
                            id,
                            function_name: name,
                            arguments: input,
                        });
                    }
                    _ => {}
                }
            }
        }

        let model = data["model"].as_str().unwrap_or(&request.model).to_owned();

        let usage_data = &data["usage"];
        let cached_tokens = usage_data["cache_read_input_tokens"].as_u64().map(|v| v as u32);
        let usage = if usage_data.is_object() {
            let prompt = usage_data["input_tokens"].as_u64().unwrap_or(0) as u32;
            let completion = usage_data["output_tokens"].as_u64().unwrap_or(0) as u32;
            TokenUsage {
                prompt_tokens: prompt,
                completion_tokens: completion,
                total_tokens: prompt + completion,
                cached_tokens,
            }
        } else {
            TokenUsage::new(0, 0)
        };

        let finish_reason_str = data["stop_reason"].as_str().unwrap_or("end_turn");
        let finish_reason = Self::parse_finish_reason(finish_reason_str);

        let thinking = None;
        let cache_info = cached_tokens.map(|ct| {
            let cost = (ct as f64 / 1_000_000.0) * self.models.first().map(|m| m.pricing.cache_read_per_million).unwrap_or(0.0);
            CacheInfo {
                cached_tokens: ct,
                cache_saved_cost: cost,
            }
        });

        Ok(CompletionResponse {
            content: if text_content.is_empty() { None } else { Some(text_content) },
            thinking,
            tool_calls,
            usage,
            model,
            finish_reason,
            latency_ms: latency,
            cache_info,
        })
    }

    async fn complete_stream(
        &self,
        request: CompletionRequest,
        options: RequestOptions,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send>>, ProviderError> {
        let (messages, system, system_blocks) = Self::convert_messages(&request.messages);

        let mut body = serde_json::json!({
            "model": request.model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "stream": true,
        });

        if let Some(temp) = request.temperature {
            body["temperature"] = Value::from(temp);
        }
        if let Some(top_p) = request.top_p {
            body["top_p"] = Value::from(top_p);
        }
        if let Some(s) = system {
            body["system"] = Value::String(s);
        } else if !system_blocks.is_empty() {
            body["system"] = Value::Array(system_blocks);
        }

        if let Some(tools) = &request.tools {
            let tool_defs: Vec<Value> = tools.iter().map(|t| {
                serde_json::json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.parameters,
                })
            }).collect();
            body["tools"] = Value::Array(tool_defs);
        }

        let mut req_builder = self
            .client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", &self.anthropic_version);

        if let Some(timeout) = options.timeout {
            req_builder = req_builder.timeout(timeout);
        }

        let resp = req_builder
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Network {
                message: e.to_string(),
                detail: Some(format!("{:?}", e.kind())),
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

        let stream = resp.bytes_stream().map(move |chunk_result| {
            let mut events = Vec::new();
            match chunk_result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    for line in text.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            if data == "[DONE]" {
                                continue;
                            }
                            if let Ok(parsed) = serde_json::from_str::<Value>(data) {
                                if let Some(event_type) = parsed["type"].as_str() {
                                    match event_type {
                                        "content_block_start" => {
                                            if let Some(content_block) = parsed["content_block"].as_object() {
                                                if let Some(index) = content_block["index"].as_u64() {
                                                    if content_block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                                                        events.push(Ok(StreamEvent::ToolCallDelta {
                                                            index: index as usize,
                                                            id: content_block["id"].as_str().map(|s| s.to_owned()),
                                                            function_name: content_block["name"].as_str().map(|s| s.to_owned()),
                                                            arguments_delta: String::new(),
                                                        }));
                                                    }
                                                }
                                            }
                                        }
                                        "content_block_delta" => {
                                            if let Some(index) = parsed["index"].as_u64() {
                                                let delta = parsed["delta"].as_object().unwrap();
                                                if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                                                    events.push(Ok(StreamEvent::ContentDelta {
                                                        delta: text.to_owned(),
                                                    }));
                                                } else if let Some(partial_json) = delta.get("partial_json").and_then(|p| p.as_str()) {
                                                    events.push(Ok(StreamEvent::ToolCallDelta {
                                                        index: index as usize,
                                                        id: None,
                                                        function_name: None,
                                                        arguments_delta: partial_json.to_owned(),
                                                    }));
                                                }
                                            }
                                        }
                                        "message_delta" => {
                                            if let Some(delta) = parsed["delta"].as_object() {
                                                if let Some(stop_reason) = delta.get("stop_reason").and_then(|s| s.as_str()) {
                                                    let finish_reason = Self::parse_finish_reason(stop_reason);
                                                    let usage_opt = parsed.get("usage").and_then(|u| {
                                                        let prompt = u["input_tokens"].as_u64().map(|v| v as u32)?;
                                                        let completion = u["output_tokens"].as_u64().map(|v| v as u32)?;
                                                        let cached = u["cache_read_input_tokens"].as_u64().map(|v| v as u32);
                                                        Some(TokenUsage {
                                                            prompt_tokens: prompt,
                                                            completion_tokens: completion,
                                                            total_tokens: prompt + completion,
                                                            cached_tokens: cached,
                                                        })
                                                    });
                                                    events.push(Ok(StreamEvent::Done {
                                                        finish_reason,
                                                        usage: usage_opt,
                                                    }));
                                                }
                                            } else if let Some(usage_data) = parsed.get("usage").and_then(|u| u.as_object()) {
                                                let prompt = usage_data["input_tokens"].as_u64().map(|v| v as u32).unwrap_or(0);
                                                let completion = usage_data["output_tokens"].as_u64().map(|v| v as u32).unwrap_or(0);
                                                let cached = usage_data["cache_read_input_tokens"].as_u64().map(|v| v as u32);
                                                events.push(Ok(StreamEvent::Usage {
                                                    usage: TokenUsage {
                                                        prompt_tokens: prompt,
                                                        completion_tokens: completion,
                                                        total_tokens: prompt + completion,
                                                        cached_tokens,
                                                    },
                                                }));
                                            }
                                        }
                                        _ => {}
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
            }
        }).flatten();

        Ok(Box::pin(stream))
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    fn name(&self) -> &str {
        &self.name
    }
}
