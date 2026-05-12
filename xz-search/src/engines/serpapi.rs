use async_trait::async_trait;
use std::time::Instant;

use crate::error::SearchError;
use crate::traits::{SearchEngine, SearchEngineInfo, SearchPricing};
use crate::types::{SearchConfig, SearchItem, SearchOptions, SearchResult};

/// SerpAPI 适配 — Google Search API 封装
#[derive(Debug)]
pub struct SerpApiEngine {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
    info: SearchEngineInfo,
}

impl SerpApiEngine {
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            base_url: "https://serpapi.com".into(),
            client: reqwest::Client::new(),
            info: SearchEngineInfo {
                name: "serpapi".into(),
                display_name: "SerpAPI (Google)".into(),
                description: "Google Search API via SerpAPI".into(),
                supported_sources: vec!["web".into(), "news".into(), "images".into()],
                max_results: 100,
                supported_regions: vec![
                    "us".into(), "cn".into(), "jp".into(), "uk".into(),
                ],
                supports_time_range: true,
                pricing: Some(SearchPricing {
                    cost_per_search: 0.01,
                }),
            },
        }
    }
}

#[async_trait]
impl SearchEngine for SerpApiEngine {
    async fn search(
        &self,
        query: &str,
        config: &SearchConfig,
        _options: &SearchOptions,
    ) -> Result<SearchResult, SearchError> {
        let start = Instant::now();

        let response = self
            .client
            .get(&self.base_url)
            .query(&[
                ("api_key", self.api_key.as_str()),
                ("engine", "google"),
                ("q", query),
                ("num", &config.max_results.to_string()),
                ("hl", config.language.as_deref().unwrap_or("en")),
                ("gl", config.region.as_deref().unwrap_or("us")),
            ])
            .send()
            .await
            .map_err(|e| SearchError::Network {
                engine: "serpapi".into(),
                message: e.to_string(),
            })?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(SearchError::Api {
                engine: "serpapi".into(),
                message: body,
            });
        }

        let result: serde_json::Value = response.json().await.map_err(|e| SearchError::Api {
            engine: "serpapi".into(),
            message: e.to_string(),
        })?;

        let organic = result["organic_results"]
            .as_array()
            .map(|arr| arr.as_slice())
            .unwrap_or(&[]);

        let mut items = Vec::new();
        for (i, r) in organic.iter().enumerate() {
            let url = r["link"].as_str().unwrap_or("").to_string();
            items.push(SearchItem {
                title: r["title"].as_str().unwrap_or("").to_string(),
                url: url.clone(),
                snippet: r["snippet"].as_str().unwrap_or("").to_string(),
                source: "serpapi".into(),
                published_at: None,
                score: 1.0 - (i as f32 / (organic.len() as f32).max(1.0)),
                domain: extract_domain(&url),
                detected_language: None,
                extracted_content: None,
            });
        }

        Ok(SearchResult {
            query: query.to_string(),
            items,
            total_results: result["search_information"]["total_results"]
                .as_u64()
                .unwrap_or(0),
            latency_ms: start.elapsed().as_millis() as u64,
            cached: false,
            engines_used: vec!["serpapi".into()],
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
