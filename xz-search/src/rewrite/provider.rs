use async_trait::async_trait;

use crate::error::SearchError;

/// LLM 查询改写提供者 trait。
/// QueryRewriter 通过此 trait 调用 LLM，不直接依赖 xz-provider。
#[async_trait]
pub trait QueryRewriteProvider: Send + Sync {
    /// 调用 LLM 重写查询，返回重写后的查询字符串
    async fn rewrite(&self, query: &str, system_prompt: &str) -> Result<String, SearchError>;
}

/// OpenAI API 实现的 LLM 查询改写提供者
#[cfg(feature = "llm-rewrite")]
pub struct OpenAiRewriteProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

#[cfg(feature = "llm-rewrite")]
impl OpenAiRewriteProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            model: "gpt-4o-mini".to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

#[cfg(feature = "llm-rewrite")]
#[async_trait]
impl QueryRewriteProvider for OpenAiRewriteProvider {
    async fn rewrite(&self, query: &str, system_prompt: &str) -> Result<String, SearchError> {
        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": format!("Query: {}", query)}
            ],
            "temperature": 0.3,
            "max_tokens": 256,
        });

        let resp = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| SearchError::Network {
                engine: "openai-rewrite".into(),
                message: e.to_string(),
            })?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(SearchError::Api {
                engine: "openai-rewrite".into(),
                message: format!("HTTP {}: {}", status.as_u16(), text),
            });
        }

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| SearchError::QueryRewrite(format!("JSON parse error: {}", e)))?;

        let content = data["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| {
                SearchError::QueryRewrite(
                    "missing choices[0].message.content in response".into(),
                )
            })?
            .to_string();

        Ok(content)
    }
}
