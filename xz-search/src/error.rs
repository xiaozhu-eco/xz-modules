use std::fmt::Debug;

/// 搜索错误
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("API 错误 [{engine}]: {message}")]
    Api { engine: String, message: String },

    #[error("网络错误 [{engine}]: {message}")]
    Network { engine: String, message: String },

    #[error("速率限制 [{engine}]: {retry_after_ms}ms 后重试")]
    RateLimit { engine: String, retry_after_ms: u64 },

    #[error("内容提取失败 [{url}]: {message}")]
    Extraction { url: String, message: String },

    #[error("配置错误: {0}")]
    Config(String),

    #[error("查询重写失败: {0}")]
    QueryRewrite(String),

    #[error("引擎不可用: {0}")]
    EngineUnavailable(String),

    #[error("所有引擎均失败")]
    AllEnginesFailed,

    #[error("超时 ({0}ms)")]
    Timeout(u64),

    #[error("认证失败 [{engine}]: {message}")]
    Auth { engine: String, message: String },

    #[error("无效 URL: {0}")]
    InvalidUrl(String),
}

impl SearchError {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            SearchError::Network { .. } | SearchError::RateLimit { .. } | SearchError::Timeout(_)
        )
    }
}
