use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use serde_json::Value;
use std::pin::Pin;

use crate::config::ProviderDefinition;
use crate::error::ProviderError;
use crate::traits::LlmProvider;
use crate::types::{
    CompletionRequest, CompletionResponse, FinishReason, Message, MessageContent,
    ModelInfo, RequestOptions, StreamEvent, TokenUsage,
};

/// 本地模型提供者（Ollama / llama.cpp）
#[derive(Debug)]
pub struct LocalProvider {
    name: String,
    base_url: String,
    models: Vec<ModelInfo>,
    client: reqwest::Client,
}

impl LocalProvider {
    pub fn new(name: String, def: &ProviderDefinition) -> Result<Self, ProviderError> {
        let base_url = def
            .base_url
            .clone()
            .unwrap_or_else(|| "http://localhost:11434".to_owned());

        let models: Vec<ModelInfo> = def
            .models
            .iter()
            .map(|cfg| {
                let mut info = ModelInfo::from(cfg.clone());
                info.provider = Some(name.clone());
                info
            })
            .collect();

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| ProviderError::Config(format!("Failed to build HTTP client: {}", e)))?;

        Ok(Self {
            name,
            base_url,
            models,
            client,
        })
    }

    fn messages_to_prompt(messages: &[Message]) -> String {
        let mut prompt = String::new();
        for msg in messages {
            let role = msg.role_str();
            let content = match msg {
                Message::System { content, .. }
                | Message::User { content, .. }
                | Message::Assistant { content, .. }
                | Message::Tool { content, .. } => match content {
                    MessageContent::Text(text) => text.clone(),
                    MessageContent::MultiPart(_) => "[multimodal content]".to_owned(),
                    MessageContent::None => String::new(),
                },
            };
            if !content.is_empty() {
                prompt.push_str(&format!("<|{}|>\n{}\n", role, content));
            }
        }
        prompt.push_str("<|assistant|>\n");
        prompt
    }
}

#[async_trait]
impl LlmProvider for LocalProvider {
    async fn complete(
        &self,
        request: CompletionRequest,
        _options: RequestOptions,
    ) -> Result<CompletionResponse, ProviderError> {
        let start = std::time::Instant::now();

        let prompt = Self::messages_to_prompt(&request.messages);

        let body = serde_json::json!({
            "model": request.model,
            "prompt": prompt,
            "stream": false,
            "options": {
                "temperature": request.temperature.unwrap_or(0.7),
                "num_predict": request.max_tokens.unwrap_or(2048),
                "stop": request.stop,
            }
        });

        let resp = self
            .client
            .post(format!("{}/api/generate", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Network {
                message: e.to_string(),
                detail: None,
            })?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Internal {
                status: status.as_u16(),
                message: text,
            });
        }

        let data: Value = resp.json().await.map_err(|e| ProviderError::Format(e.to_string()))?;
        let latency = start.elapsed().as_millis() as u64;

        let content = data["response"].as_str().map(|s| s.to_owned());
        let model = data["model"].as_str().unwrap_or("unknown").to_owned();

        let usage = if data.get("eval_count").is_some() {
            TokenUsage {
                prompt_tokens: data["prompt_eval_count"].as_u64().unwrap_or(0) as u32,
                completion_tokens: data["eval_count"].as_u64().unwrap_or(0) as u32,
                total_tokens: 0,
                cached_tokens: None,
            }
        } else {
            TokenUsage::new(0, 0)
        };

        Ok(CompletionResponse {
            content,
            thinking: None,
            tool_calls: Vec::new(),
            usage,
            model,
            finish_reason: FinishReason::Stop,
            latency_ms: latency,
            cache_info: None,
        })
    }

    async fn complete_stream(
        &self,
        request: CompletionRequest,
        _options: RequestOptions,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send>>, ProviderError>
    {
        let prompt = Self::messages_to_prompt(&request.messages);

        let body = serde_json::json!({
            "model": request.model,
            "prompt": prompt,
            "stream": true,
            "options": {
                "temperature": request.temperature.unwrap_or(0.7),
                "num_predict": request.max_tokens.unwrap_or(2048),
            }
        });

        let resp = self
            .client
            .post(format!("{}/api/generate", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Network {
                message: e.to_string(),
                detail: None,
            })?;

        let status_code = resp.status().as_u16();
        if status_code != 200 {
            let text = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Internal {
                status: status_code,
                message: text,
            });
        }

        let stream = resp.bytes_stream().map(|chunk_result| match chunk_result {
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes);
                let mut events = Vec::new();
                for line in text.lines() {
                    if let Ok(parsed) = serde_json::from_str::<Value>(line) {
                        let delta = parsed["response"].as_str().unwrap_or("").to_owned();
                        let done = parsed["done"].as_bool().unwrap_or(false);

                        if !delta.is_empty() {
                            events.push(Ok(StreamEvent::ContentDelta { delta }));
                        }

                        if done {
                            let usage = if parsed.get("eval_count").is_some() {
                                Some(TokenUsage {
                                    prompt_tokens: parsed["prompt_eval_count"]
                                        .as_u64()
                                        .unwrap_or(0) as u32,
                                    completion_tokens: parsed["eval_count"].as_u64().unwrap_or(0) as u32,
                                    total_tokens: 0,
                                    cached_tokens: None,
                                })
                            } else {
                                None
                            };

                            events.push(Ok(StreamEvent::Done {
                                finish_reason: FinishReason::Stop,
                                usage,
                            }));
                        }
                    }
                }
                futures::stream::iter(events)
            }
            Err(e) => futures::stream::iter(vec![Err(ProviderError::Network {
                message: e.to_string(),
                detail: None,
            })]),
        })
        .flatten()
        .boxed();

        Ok(stream)
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    fn name(&self) -> &str {
        &self.name
    }
}
