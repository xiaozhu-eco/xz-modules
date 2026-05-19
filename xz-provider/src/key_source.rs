use async_trait::async_trait;

use crate::error::ProviderError;

/// 用于获取 LLM Provider API Key 的可插拔 trait。
///
/// 不同场景下 API Key 来源不同：
/// - 配置文件（`ConfigKeySource`）
/// - 用户自己提供（`UserKeySource`）
///
/// 所有实现均通过本 trait 统一接口接入 `ProviderRouter`。
#[async_trait]
pub trait KeySource: Send + Sync {
    /// 获取 API Key。
    ///
    /// 对于 ConfigKeySource：直接从配置中读取。
    async fn get_api_key(&self) -> Result<String, ProviderError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestSource {
        key: String,
    }

    #[async_trait]
    impl KeySource for TestSource {
        async fn get_api_key(&self) -> Result<String, ProviderError> {
            Ok(self.key.clone())
        }
    }

    #[tokio::test]
    async fn trait_is_object_safe() {
        use std::sync::Arc;

        let source: Arc<dyn KeySource> = Arc::new(TestSource {
            key: "sk-test".into(),
        });
        let api_key = source.get_api_key().await.ok();

        assert_eq!(api_key.as_deref(), Some("sk-test"));
    }

    struct ErrSource;

    #[async_trait]
    impl KeySource for ErrSource {
        async fn get_api_key(&self) -> Result<String, ProviderError> {
            Err(ProviderError::KeySource("test error".into()))
        }
    }

    #[tokio::test]
    async fn trait_returns_error() {
        let source = ErrSource;

        assert!(source.get_api_key().await.is_err());
    }
}
