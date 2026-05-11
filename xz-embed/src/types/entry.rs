use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 向量条目 — 存储的最小单元
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEntry {
    /// 全局唯一标识（建议使用 UUID v7）
    pub id: String,
    /// 浮点向量
    pub vector: Vec<f32>,
    /// 键值对元数据（用于过滤和聚合）
    pub metadata: HashMap<String, String>,
    /// 原始文本内容（可选，用于调试和混合检索）
    pub content: Option<String>,
    /// 创建时间（epoch milliseconds）
    pub created_at: u64,
    /// 过期时间（epoch milliseconds，None = 永不过期）
    pub expires_at: Option<u64>,
    /// 来源通道标签（如 "seed", "semantic", "manual"）
    pub channel: Option<String>,
}

/// 搜索结果 — 向量搜索返回的条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// 对应 VectorEntry.id
    pub id: String,
    /// 相似度分数（cosine 相似度，范围 [-1, 1]）
    pub score: f32,
    /// 元数据
    pub metadata: HashMap<String, String>,
    /// 原始文本内容
    pub content: Option<String>,
    /// 来源通道
    pub channel: Option<String>,
}
