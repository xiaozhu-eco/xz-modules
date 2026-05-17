use serde::{Deserialize, Serialize};

use crate::error::RerankError;

/// 信号权重配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalWeights {
    /// 关键词重叠度权重
    pub keyword_overlap: f32,
    /// 向量相似度权重
    pub vector_similarity: f32,
    /// 元数据匹配权重
    pub metadata_match: f32,
    /// 内容质量权重
    pub content_quality: f32,
    /// 时间近因性权重
    pub recency: f32,
}

impl Default for SignalWeights {
    fn default() -> Self {
        Self {
            keyword_overlap: 0.30,
            vector_similarity: 0.25,
            metadata_match: 0.20,
            content_quality: 0.10,
            recency: 0.15,
        }
    }
}

impl SignalWeights {
    /// 验证权重和为 1.0
    pub fn validate(&self) -> Result<(), RerankError> {
        let sum = self.keyword_overlap
            + self.vector_similarity
            + self.metadata_match
            + self.content_quality
            + self.recency;

        if (sum - 1.0).abs() > 0.01 {
            return Err(RerankError::WeightSumInvalid(sum));
        }
        Ok(())
    }

    /// 按名称查找权重（用于信号名称映射）
    pub fn get_weight_by_name(&self, name: &str) -> f32 {
        match name {
            "keyword_overlap" => self.keyword_overlap,
            "vector_similarity" => self.vector_similarity,
            "metadata_match" => self.metadata_match,
            "content_quality" => self.content_quality,
            "recency" => self.recency,
            _ => 0.0,
        }
    }

    /// 自动归一化
    pub fn normalize(&self) -> Self {
        let sum = self.keyword_overlap
            + self.vector_similarity
            + self.metadata_match
            + self.content_quality
            + self.recency;

        if sum == 0.0 {
            return Self::default();
        }

        Self {
            keyword_overlap: self.keyword_overlap / sum,
            vector_similarity: self.vector_similarity / sum,
            metadata_match: self.metadata_match / sum,
            content_quality: self.content_quality / sum,
            recency: self.recency / sum,
        }
    }
}
