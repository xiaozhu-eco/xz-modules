use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Instant;

use crate::error::SearchError;
use crate::traits::{SearchEngine, SearchEngineInfo, SearchPricing};
use crate::types::{SearchConfig, SearchItem, SearchOptions, SearchResult};

/// Tavily Search API 适配
///
/// Tavily 专为 AI Agent 设计，返回高质量、LLM 友好的搜索结果。
#[derive(Debug)]
pub struct TavilyEngine {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
    info: SearchEngineInfo,
}

impl TavilyEngine {
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            base_url: "https://api.tavily.com".into(),
            client: reqwest::Client::new(),
            info: SearchEngineInfo {
                name: "tavily".into(),
                display_name: "Tavily Search".into(),
                description: "AI-optimized search API".into(),
                supported_sources: vec!["web".into(), "news".into()],
                max_results: 20,
                supported_regions: vec!["global".into()],
                supports_time_range: true,
                pricing: Some(SearchPricing {
                    cost_per_search: 0.005,
                }),
            },
        }
    }

    fn to_tavily_params(&self, query: &str, config: &SearchConfig) -> serde_json::Value {
        let search_depth = if config.max_results > 5 {
            "advanced"
        } else {
            "basic"
        };

        serde_json::json!({
            "api_key": self.api_key,
            "query": query,
            "search_depth": search_depth,
            "max_results": config.max_results,
            "include_answer": config.auto_extract,
            "include_raw_content": false,
            "include_domains": [],
            "exclude_domains": [],
        })
    }

    fn map_to_search_items(&self, raw_results: &[serde_json::Value]) -> Vec<SearchItem> {
        raw_results
            .iter()
            .map(|r| {
                let url = r["url"].as_str().unwrap_or("").to_string();
                SearchItem {
                    title: r["title"].as_str().unwrap_or("").to_string(),
                    url: url.clone(),
                    snippet: r["content"].as_str().unwrap_or("").to_string(),
                    source: "tavily".into(),
                    published_at: None,
                    score: r["score"].as_f64().unwrap_or(0.5) as f32,
                    domain: extract_domain(&url),
                    detected_language: None,
                    extracted_content: None,
                }
            })
            .collect()
    }
}

#[async_trait]
impl SearchEngine for TavilyEngine {
    async fn search(
        &self,
        query: &str,
        config: &SearchConfig,
        _options: &SearchOptions,
    ) -> Result<SearchResult, SearchError> {
        let start = Instant::now();

        let request_body = self.to_tavily_params(query, config);

        let response = self
            .client
            .post(format!("{}/search", self.base_url))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| SearchError::Network {
                engine: "tavily".into(),
                message: e.to_string(),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            if status.as_u16() == 401 {
                return Err(SearchError::Auth {
                    engine: "tavily".into(),
                    message: body,
                });
            }
            if status.as_u16() == 429 {
                return Err(SearchError::RateLimit {
                    engine: "tavily".into(),
                    retry_after_ms: 1000,
                });
            }
            return Err(SearchError::Api {
                engine: "tavily".into(),
                message: format!("HTTP {status}: {body}"),
            });
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| SearchError::Api {
                engine: "tavily".into(),
                message: e.to_string(),
            })?;

        let raw = result["results"]
            .as_array()
            .map(|arr| arr.as_slice())
            .unwrap_or(&[]);

        let items = self.map_to_search_items(raw);
        let total = items.len() as u64;

        Ok(SearchResult {
            query: query.to_string(),
            items,
            total_results: total,
            latency_ms: start.elapsed().as_millis() as u64,
            cached: false,
            engines_used: vec!["tavily".into()],
            rewritten_query: None,
        })
    }

    fn engine_info(&self) -> &SearchEngineInfo {
        &self.info
    }
}

fn extract_domain(url: &str) -> String {
    url.trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or(url)
        .to_string()
}
