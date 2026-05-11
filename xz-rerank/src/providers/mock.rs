use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use crate::error::RerankError;
use crate::traits::{Reranker, RerankerBackendType, RerankerInfo, RerankerPricing};
use crate::types::{RerankCandidate, RerankConfig, RerankHit, RerankResult, RerankStats};

/// 测试用 Mock Reranker
#[derive(Debug)]
pub struct MockReranker {
    info: RerankerInfo,
    mock_result: Mutex<Option<RerankResult>>,
    should_error: Mutex<Option<RerankError>>,
}

impl MockReranker {
    pub fn new(name: &str) -> Self {
        Self {
            info: RerankerInfo {
                name: name.to_string(),
                display_name: format!("Mock {name}"),
                backend_type: RerankerBackendType::Local,
                supports_batch: true,
                max_candidates: None,
                pricing: None,
            },
            mock_result: Mutex::new(None),
            should_error: Mutex::new(None),
        }
    }

    pub fn set_result(&mut self, result: RerankResult) {
        *self.mock_result.get_mut().unwrap() = Some(result);
    }

    pub fn set_error(&mut self, error: RerankError) {
        *self.should_error.get_mut().unwrap() = Some(error);
    }
}

#[async_trait]
impl Reranker for MockReranker {
    async fn rerank(
        &self,
        query: &str,
        candidates: Vec<RerankCandidate>,
        config: &RerankConfig,
    ) -> Result<RerankResult, RerankError> {
        if let Some(ref err) = *self.should_error.lock().unwrap() {
            return Err(RerankError::Engine(format!("Mock error: {err}")));
        }

        if let Some(ref result) = *self.mock_result.lock().unwrap() {
            return Ok(result.clone());
        }

        // 默认行为：原样返回，分数 = retrieval_score or 0.5
        let now = Instant::now();
        let hits: Vec<RerankHit> = candidates
            .iter()
            .take(config.top_k)
            .map(|c| RerankHit {
                candidate_id: c.id.clone(),
                score: c.retrieval_score.unwrap_or(0.5),
                score_breakdown: None,
                candidate: c.clone(),
            })
            .collect();

        let n = hits.len();
        Ok(RerankResult {
            hits,
            stats: RerankStats {
                total_candidates: candidates.len(),
                filtered_out: 0,
                max_score: 1.0,
                min_score: 0.0,
                avg_score: 0.5,
                median_score: 0.5,
                signal_timings: HashMap::new(),
            },
            reranker: self.info.name.clone(),
            latency_ms: now.elapsed().as_millis() as u64,
        })
    }

    fn reranker_info(&self) -> &RerankerInfo {
        &self.info
    }
}
