use async_trait::async_trait;

use crate::error::RerankError;
use crate::traits::Reranker;
use crate::types::{RerankCandidate, RerankConfig, RerankResult};

/// 多阶段重排序器 — 粗排 → 精排
///
/// 第一阶段（粗排）：快速过滤大量候选项
/// 第二阶段（精排）：对少量候选项精确打分
pub struct MultiStageReranker<S1: Reranker, S2: Reranker> {
    stage1: S1,
    stage2: S2,
    /// 第一阶段后保留的候选项数
    top_k_after_stage1: usize,
}

impl<S1: Reranker, S2: Reranker> MultiStageReranker<S1, S2> {
    pub fn new(stage1: S1, stage2: S2, top_k_after_stage1: usize) -> Self {
        Self {
            stage1,
            stage2,
            top_k_after_stage1,
        }
    }
}

impl<S1: Reranker, S2: Reranker> std::fmt::Debug for MultiStageReranker<S1, S2> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultiStageReranker")
            .field("stage1", &self.stage1.reranker_info().name)
            .field("stage2", &self.stage2.reranker_info().name)
            .field("top_k_after_stage1", &self.top_k_after_stage1)
            .finish()
    }
}

#[async_trait]
impl<S1: Reranker, S2: Reranker> Reranker for MultiStageReranker<S1, S2> {
    async fn rerank(
        &self,
        query: &str,
        candidates: Vec<RerankCandidate>,
        config: &RerankConfig,
    ) -> Result<RerankResult, RerankError> {
        // 阶段1：粗排
        let stage1_config = RerankConfig {
            top_k: self.top_k_after_stage1,
            ..config.clone()
        };
        let stage1_result = self.stage1.rerank(query, candidates, &stage1_config).await?;

        // 提取 top_k 候选项
        let top_candidates: Vec<RerankCandidate> = stage1_result
            .hits
            .into_iter()
            .take(self.top_k_after_stage1)
            .map(|h| h.candidate)
            .collect();

        // 阶段2：精排
        self.stage2.rerank(query, top_candidates, config).await
    }

    fn reranker_info(&self) -> &crate::traits::RerankerInfo {
        // 返回 stage2 的信息作为对外标识
        self.stage2.reranker_info()
    }
}
