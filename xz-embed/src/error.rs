use std::fmt::Debug;

/// 嵌入模型错误
#[derive(Debug, thiserror::Error)]
pub enum EmbedError {
    #[error("嵌入模型错误: {0}")]
    Model(String),

    #[error("批次大小超限: {actual} > {limit}")]
    BatchSizeExceeded { actual: usize, limit: usize },

    #[error("输入超长: token 数 {actual} > {limit}")]
    InputTooLong { actual: usize, limit: usize },

    #[error("维度不匹配: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    #[error("网络错误: {0}")]
    Network(String),

    #[error("限流: {retry_after_ms}ms 后重试")]
    RateLimit { retry_after_ms: u64 },

    #[error("认证失败: {0}")]
    Auth(String),

    #[error("配置错误: {0}")]
    Config(String),

    #[error("批次为空")]
    EmptyBatch,
}

impl EmbedError {
    pub fn is_retryable(&self) -> bool {
        matches!(self, EmbedError::Network(_) | EmbedError::RateLimit { .. })
    }
}

/// 向量存储错误
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("数据库错误: {0}")]
    Database(String),

    #[error("维度不匹配: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    #[error("条目未找到: {0}")]
    NotFound(String),

    #[error("存储已关闭")]
    Closed,

    #[error("序列化错误: {0}")]
    Serialization(String),

    #[error("索引重建中")]
    Indexing,

    #[error("存储已满: {used}/{capacity} bytes")]
    Full { used: u64, capacity: u64 },
}
