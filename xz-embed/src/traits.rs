use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use crate::error::{EmbedError, StoreError};
use crate::types::{MetadataFilter, SearchResult, StoreStats, VectorEntry};

/// 文本截断策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TruncationStrategy {
    /// 直接报错（严格要求）
    Error,
    /// 从开头截断（保留前 N 个 token）
    TruncateStart,
    /// 从末尾截断（保留后 N 个 token）
    TruncateEnd,
    /// 保留首尾各 N/2 个 token（适合 LLM 输出）
    TruncateBoth,
}

/// 嵌入模型定价信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedPricing {
    /// 每百万 token 价格（输入）
    pub input_per_million: f64,
}

/// 嵌入模型元信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedModelInfo {
    pub name: String,
    pub display_name: String,
    /// 支持的维度列表（None 表示不支持维度选择）
    pub supported_dimensions: Option<Vec<usize>>,
    /// 当前维度
    pub current_dimension: usize,
    /// 最大输入 token 限制
    pub max_input_tokens: usize,
    /// 最大批次大小
    pub max_batch_size: usize,
    /// 定价信息
    pub pricing: EmbedPricing,
}

/// 统一的文本向量嵌入接口
#[async_trait]
pub trait EmbeddingModel: Send + Sync + Debug {
    /// 核心嵌入方法。对一批文本生成向量。
    async fn embed(&self, input: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError>;

    /// 便捷方法：对单个文本生成向量
    async fn embed_single(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        let mut results = self.embed(&[text]).await?;
        if results.is_empty() {
            return Err(EmbedError::Model("embed_single 返回空结果".into()));
        }
        Ok(results.remove(0))
    }

    /// 模型信息
    fn model_info(&self) -> &EmbedModelInfo;

    /// 最大批次大小
    fn max_batch_size(&self) -> usize;

    /// 向量维度
    fn dimensions(&self) -> usize;
}

/// 维度约简策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DimensionReducer {
    /// API 原生支持（OpenAI dimensions 参数）
    Native(usize),
    /// 截断（取前 N 维）
    Truncate(usize),
}

impl DimensionReducer {
    /// 应用降维
    pub fn reduce(&self, vectors: &mut [Vec<f32>]) {
        match self {
            DimensionReducer::Native(_) => {
                // 原生降维由 API 端处理，此处无需操作
            }
            DimensionReducer::Truncate(n) => {
                for v in vectors.iter_mut() {
                    v.truncate(*n);
                }
            }
        }
    }
}

/// 向量存储抽象 — 可插拔后端
#[async_trait]
pub trait VectorStore: Send + Sync + Debug {
    /// 插入单条向量
    async fn insert(&self, entry: VectorEntry) -> Result<(), StoreError>;

    /// 批量插入向量（推荐的高吞吐写入路径）
    async fn insert_batch(&self, entries: Vec<VectorEntry>) -> Result<(), StoreError>;

    /// 相似度搜索（cosine 相似度，返回 Top-K）
    async fn search(&self, query: &[f32], limit: usize) -> Result<Vec<SearchResult>, StoreError>;

    /// 带元数据过滤的相似度搜索
    async fn search_with_filter(
        &self,
        query: &[f32],
        filter: &MetadataFilter,
        limit: usize,
    ) -> Result<Vec<SearchResult>, StoreError>;

    /// 按 ID 批量删除
    async fn delete(&self, ids: &[String]) -> Result<usize, StoreError>;

    /// 按元数据过滤条件删除
    async fn delete_by_filter(&self, filter: &MetadataFilter) -> Result<usize, StoreError>;

    /// 清空存储
    async fn clear(&self) -> Result<(), StoreError>;

    /// 存储中的总条目数
    async fn count(&self) -> Result<usize, StoreError>;

    /// 创建/重建索引（后台线程，不阻塞写入）
    async fn rebuild_index(&self) -> Result<(), StoreError>;

    /// 存储统计信息
    async fn stats(&self) -> Result<StoreStats, StoreError>;
}

/// VectorStore 生命周期 trait
#[async_trait]
pub trait StoreLifecycle: VectorStore {
    /// 初始化存储（创建表结构等）
    async fn initialize(&self) -> Result<(), StoreError>;

    /// 优雅关闭
    async fn close(&self) -> Result<(), StoreError>;

    /// 强制持久化检查点
    async fn checkpoint(&self) -> Result<(), StoreError>;

    /// 是否健康
    async fn health_check(&self) -> Result<bool, StoreError>;
}

/// BM25 关键词检索 trait
pub trait KeywordSearch: Send + Sync {
    /// 全文检索，返回 (document_id, bm25_score)
    fn search(&self, query: &str, limit: usize) -> Vec<(String, f32)>;
}
