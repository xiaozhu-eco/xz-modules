use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use crate::error::RerankError;
use crate::types::{RerankCandidate, RerankConfig, RerankResult};

/// 重排序器后端类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RerankerBackendType {
    Local,
    Remote,
    Plugin,
}

/// 重排序器信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankerInfo {
    pub name: String,
    pub display_name: String,
    pub backend_type: RerankerBackendType,
    pub supports_batch: bool,
    pub max_candidates: Option<usize>,
    pub pricing: Option<RerankerPricing>,
}

/// 重排序器定价
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankerPricing {
    pub cost_per_search: f64,
}

/// 统一的重排序接口
#[async_trait]
pub trait Reranker: Send + Sync + Debug {
    /// 对候选项重新排序
    async fn rerank(
        &self,
        query: &str,
        candidates: Vec<RerankCandidate>,
        config: &RerankConfig,
    ) -> Result<RerankResult, RerankError>;

    /// 对候选项重新排序（简化版，使用默认配置）
    async fn rerank_default(
        &self,
        query: &str,
        candidates: Vec<RerankCandidate>,
    ) -> Result<RerankResult, RerankError> {
        self.rerank(query, candidates, &RerankConfig::default())
            .await
    }

    /// 重排序器信息
    fn reranker_info(&self) -> &RerankerInfo;
}

/// 信号插件 trait — 可自定义打分信号
#[async_trait]
pub trait SignalPlugin: Send + Sync + Debug {
    /// 信号名称（用于分数分解）
    fn name(&self) -> &str;

    /// 对单个候选项打分（返回 [0, 1] 区间的分数）
    async fn score(
        &self,
        query: &str,
        candidate: &RerankCandidate,
    ) -> Result<f32, RerankError>;

    /// 批量打分
    async fn score_batch(
        &self,
        query: &str,
        candidates: &[RerankCandidate],
    ) -> Result<Vec<f32>, RerankError> {
        use futures::future::join_all;

        let futures: Vec<_> = candidates.iter().map(|candidate| self.score(query, candidate)).collect();
        let results = join_all(futures).await;

        let mut scores = Vec::with_capacity(results.len());
        for result in results {
            scores.push(result?);
        }

        Ok(scores)
    }
}

/// 近因性衰减函数
pub trait RecencyFunction: Send + Sync + Debug {
    fn decay(&self, age_seconds: f64) -> f32;
}

/// Linear recency decay: score = max(0, 1 - age / max_age)
#[derive(Debug, Clone)]
pub struct LinearRecencyDecay {
    pub max_age_seconds: f64,
}

impl LinearRecencyDecay {
    pub fn new(max_age_seconds: f64) -> Self {
        Self { max_age_seconds }
    }
}

impl RecencyFunction for LinearRecencyDecay {
    fn decay(&self, age_seconds: f64) -> f32 {
        if age_seconds <= 0.0 {
            1.0
        } else {
            (1.0 - age_seconds / self.max_age_seconds).max(0.0) as f32
        }
    }
}

/// Exponential recency decay: score = e^(-rate * age)
#[derive(Debug, Clone)]
pub struct ExponentialRecencyDecay {
    pub decay_rate: f64,
}

impl ExponentialRecencyDecay {
    pub fn new(decay_rate: f64) -> Self {
        Self { decay_rate }
    }
}

impl RecencyFunction for ExponentialRecencyDecay {
    fn decay(&self, age_seconds: f64) -> f32 {
        if age_seconds <= 0.0 {
            1.0
        } else {
            (-self.decay_rate * age_seconds).exp() as f32
        }
    }
}
