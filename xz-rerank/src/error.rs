use std::fmt::Debug;

/// 重排序错误
#[derive(Debug, thiserror::Error)]
pub enum RerankError {
    #[error("候选项为空")]
    EmptyCandidates,

    #[error("候选项超限: {actual} > {limit}")]
    TooManyCandidates { actual: usize, limit: usize },

    #[error("重排序引擎错误: {0}")]
    Engine(String),

    #[error("信号 '{0}' 错误: {1}")]
    SignalError(String, String),

    #[error("配置错误: {0}")]
    Config(String),

    #[error("API 错误: {0}")]
    Api(String),

    #[error("权重和不为 1.0: sum = {0}")]
    WeightSumInvalid(f32),

    #[error("缺少 query embedding（vector_similarity 信号需要外部传入）")]
    MissingQueryEmbedding,
}

impl RerankError {
    pub fn is_retryable(&self) -> bool {
        matches!(self, RerankError::Api(_) | RerankError::Engine(_))
    }
}
