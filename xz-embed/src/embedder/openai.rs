use async_trait::async_trait;
use std::fmt::Debug;

use crate::config::{EmbedConfig, ModelConfig};
use crate::error::EmbedError;
use crate::traits::{EmbedModelInfo, EmbedPricing, EmbeddingModel};

/// OpenAI Embedding 适配器
#[derive(Debug)]
pub struct OpenAiEmbedder {
    info: EmbedModelInfo,
    api_key: String,
    base_url: String,
    model_name: String,
    dimensions: usize,
    client: reqwest::Client,
}

impl OpenAiEmbedder {
    /// 从配置创建 OpenAI Embedder
    pub fn from_config(config: &EmbedConfig) -> Result<Self, EmbedError> {
        let model_config = config.default_model_config()?;
        Self::new(config, model_config)
    }

    /// 从环境变量创建（使用 OPENAI_API_KEY）
    pub fn from_env() -> Result<Self, EmbedError> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| EmbedError::Auth("OPENAI_API_KEY 未设置".into()))?;
        let model_name = std::env::var("OPENAI_EMBED_MODEL")
            .unwrap_or_else(|_| "text-embedding-3-small".to_string());

        let info = EmbedModelInfo {
            name: model_name.clone(),
            display_name: "OpenAI Embedding".into(),
            supported_dimensions: Some(vec![512, 1536, 3072]),
            current_dimension: 1536,
            max_input_tokens: 8191,
            max_batch_size: 2048,
            pricing: EmbedPricing {
                input_per_million: 0.02,
            },
        };

        Ok(Self {
            info,
            api_key,
            base_url: "https://api.openai.com/v1".into(),
            model_name,
            dimensions: 1536,
            client: reqwest::Client::new(),
        })
    }

    fn new(_config: &EmbedConfig, model_config: &ModelConfig) -> Result<Self, EmbedError> {
        let api_key = model_config
            .api_key
            .clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| EmbedError::Auth("API key 未配置".into()))?;

        let info = EmbedModelInfo {
            name: model_config.model.clone().unwrap_or_else(|| model_config.provider.clone()),
            display_name: format!("OpenAI {}", model_config.provider),
            supported_dimensions: Some(vec![512, 1536, 3072]),
            current_dimension: model_config.dimensions,
            max_input_tokens: model_config.max_input_tokens,
            max_batch_size: model_config.max_batch_size,
            pricing: EmbedPricing {
                input_per_million: model_config.input_per_million,
            },
        };

        Ok(Self {
            info,
            api_key,
            base_url: model_config.base_url.clone().unwrap_or_else(|| "https://api.openai.com/v1".into()),
            model_name: model_config.model.clone().unwrap_or_else(|| model_config.provider.clone()),
            dimensions: model_config.dimensions,
            client: reqwest::Client::new(),
        })
    }
}

#[async_trait]
impl EmbeddingModel for OpenAiEmbedder {
    async fn embed(&self, input: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError> {
        if input.is_empty() {
            return Err(EmbedError::EmptyBatch);
        }

        if input.len() > self.max_batch_size() {
            return Err(EmbedError::BatchSizeExceeded {
                actual: input.len(),
                limit: self.max_batch_size(),
            });
        }

        let request_body = serde_json::json!({
            "model": self.model_name,
            "input": input,
            "dimensions": self.dimensions,
        });

        let response = self
            .client
            .post(format!("{}/embeddings", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| EmbedError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            if status.as_u16() == 429 {
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(1000);
                return Err(EmbedError::RateLimit {
                    retry_after_ms: retry_after,
                });
            }
            let body = response.text().await.unwrap_or_default();

            if status.as_u16() == 401 {
                return Err(EmbedError::Auth(body));
            }
            return Err(EmbedError::Model(format!("HTTP {status}: {body}")));
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| EmbedError::Network(e.to_string()))?;

        let data = result["data"]
            .as_array()
            .ok_or_else(|| EmbedError::Model("响应格式错误：缺少 data 字段".into()))?;

        let mut vectors = Vec::with_capacity(data.len());
        for item in data {
            let embedding: Vec<f32> = item["embedding"]
                .as_array()
                .ok_or_else(|| EmbedError::Model("响应格式错误：缺少 embedding 字段".into()))?
                .iter()
                .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                .collect();

            if embedding.len() != self.dimensions {
                return Err(EmbedError::DimensionMismatch {
                    expected: self.dimensions,
                    actual: embedding.len(),
                });
            }
            vectors.push(embedding);
        }

        Ok(vectors)
    }

    fn model_info(&self) -> &EmbedModelInfo {
        &self.info
    }

    fn max_batch_size(&self) -> usize {
        self.info.max_batch_size
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }
}

impl OpenAiEmbedder {
    /// 创建指定维度的 Embedder
    pub fn with_dimensions(mut self, dimensions: usize) -> Result<Self, EmbedError> {
        if let Some(ref supported) = self.info.supported_dimensions {
            if !supported.contains(&dimensions) {
                return Err(EmbedError::Config(format!(
                    "不支持的维度 {dimensions}，支持的维度: {supported:?}"
                )));
            }
        }
        self.dimensions = dimensions;
        self.info.current_dimension = dimensions;
        Ok(self)
    }
}
