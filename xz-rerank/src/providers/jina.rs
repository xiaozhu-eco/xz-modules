use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Instant;

use crate::error::RerankError;
use crate::traits::{Reranker, RerankerBackendType, RerankerInfo, RerankerPricing};
use crate::types::{RerankCandidate, RerankConfig, RerankHit, RerankResult, RerankStats};

/// Jina Reranker API 适配
#[derive(Debug)]
pub struct JinaReranker {
    api_key: String,
    model: String,
    base_url: String,
    client: reqwest::Client,
    info: RerankerInfo,
}

impl JinaReranker {
    pub fn new(api_key: &str) -> Result<Self, RerankError> {
        Ok(Self {
            api_key: api_key.to_string(),
            model: "jina-reranker-v2-base-multilingual".into(),
            base_url: "https://api.jina.ai/v1".into(),
            client: reqwest::Client::new(),
            info: RerankerInfo {
                name: "jina".into(),
                display_name: "Jina Reranker".into(),
                backend_type: RerankerBackendType::Remote,
                supports_batch: true,
                max_candidates: Some(500),
                pricing: Some(RerankerPricing {
                    cost_per_search: 0.0005,
                }),
            },
        })
    }

    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }
}

#[async_trait]
impl Reranker for JinaReranker {
    async fn rerank(
        &self,
        query: &str,
        candidates: Vec<RerankCandidate>,
        config: &RerankConfig,
    ) -> Result<RerankResult, RerankError> {
        let start = Instant::now();

        if candidates.is_empty() {
            return Err(RerankError::EmptyCandidates);
        }

        if let Some(max_c) = self.info.max_candidates {
            if candidates.len() > max_c {
                return Err(RerankError::TooManyCandidates {
                    actual: candidates.len(),
                    limit: max_c,
                });
            }
        }

        let documents: Vec<&str> = candidates.iter().map(|c| c.content.as_str()).collect();
        let top_n = config.top_k.min(documents.len());

        let request_body = serde_json::json!({
            "model": self.model,
            "query": query,
            "documents": documents,
            "top_n": top_n,
        });

        let response = self
            .client
            .post(format!("{}/rerank", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| RerankError::Api(e.to_string()))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(RerankError::Api(format!("Jina API error: {body}")));
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| RerankError::Api(e.to_string()))?;

        let raw_results = result["results"]
            .as_array()
            .ok_or_else(|| RerankError::Api("响应格式错误：缺少 results".into()))?;

        let mut hits = Vec::new();

        for item in raw_results {
            let idx = item["index"].as_u64().unwrap_or(0) as usize;
            let relevance_score = item["relevance_score"].as_f64().unwrap_or(0.0) as f32;

            if idx < candidates.len() {
                hits.push(RerankHit {
                    candidate_id: candidates[idx].id.clone(),
                    score: relevance_score,
                    score_breakdown: None,
                    candidate: candidates[idx].clone(),
                });
            }
        }

        let min_score = config.min_score.unwrap_or(0.0);
        let before_filter = hits.len();
        hits.retain(|h| h.score >= min_score);
        let filtered_out = before_filter - hits.len();

        let n = hits.len();
        let max_score = hits.first().map(|h| h.score).unwrap_or(0.0);
        let min_score_final = hits.last().map(|h| h.score).unwrap_or(0.0);
        let avg_score = if n > 0 {
            hits.iter().map(|h| h.score).sum::<f32>() / n as f32
        } else {
            0.0
        };
        let mut sorted_scores: Vec<f32> = hits.iter().map(|h| h.score).collect();
        sorted_scores.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median_score = if n > 0 { sorted_scores[n / 2] } else { 0.0 };

        Ok(RerankResult {
            hits,
            stats: RerankStats {
                total_candidates: candidates.len(),
                filtered_out,
                max_score,
                min_score: min_score_final,
                avg_score,
                median_score,
                signal_timings: HashMap::new(),
            },
            reranker: self.info.name.clone(),
            latency_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn reranker_info(&self) -> &RerankerInfo {
        &self.info
    }
}
