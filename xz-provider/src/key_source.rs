use crate::error::ProviderError;

/// 用于获取 LLM Provider API Key 的可插拔 trait。
///
/// 不同场景下 API Key 来源不同：
/// - 配置文件（`ConfigKeySource`）
/// - 用户自己提供（`UserKeySource`）
/// - 小竹官网租赁（`LeasedKeySource`）
///
/// 所有实现均通过本 trait 统一接口接入 `ProviderRouter`。
pub trait KeySource: Send + Sync {
    /// 获取 API Key。
    ///
    /// 对于 ConfigKeySource：直接从配置中读取。
    /// 对于 LeasedKeySource：可能触发 key 租赁请求。
    fn get_api_key(&self) -> Result<String, ProviderError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestSource {
        key: String,
    }

    impl KeySource for TestSource {
        fn get_api_key(&self) -> Result<String, ProviderError> {
            Ok(self.key.clone())
        }
    }

    #[test]
    fn trait_is_object_safe() {
        use std::sync::Arc;
        let source: Arc<dyn KeySource> = Arc::new(TestSource {
            key: "sk-test".into(),
        });
        assert_eq!(source.get_api_key().unwrap(), "sk-test");
    }

    struct ErrSource;

    impl KeySource for ErrSource {
        fn get_api_key(&self) -> Result<String, ProviderError> {
            Err(ProviderError::KeySource("test error".into()))
        }
    }

    #[test]
    fn trait_returns_error() {
        let source = ErrSource;
        assert!(source.get_api_key().is_err());
    }
}
