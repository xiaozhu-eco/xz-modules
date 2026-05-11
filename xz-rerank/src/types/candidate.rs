use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 重排序候选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankCandidate {
    /// 候选项唯一标识
    pub id: String,
    /// 候选项文本内容
    pub content: String,
    /// 键值对元数据
    pub metadata: HashMap<String, String>,
    /// 来自检索阶段的原始得分（可选）
    pub retrieval_score: Option<f32>,
    /// 来源通道
    pub channel: Option<String>,
    /// 创建时间（epoch ms）
    pub created_at: Option<u64>,
    /// 向量（可选，用于 vector_similarity 信号）
    pub embedding: Option<Vec<f32>>,
}
