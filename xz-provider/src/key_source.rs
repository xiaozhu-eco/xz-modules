use std::sync::Arc;

use tokio::sync::Mutex;
use xz_auth_client::{LeasedKey, XiaozhuClient};

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

/// 通过小竹积分租赁 API Key 的 KeySource 实现。
///
/// 内部缓存租赁的 key，过期后自动重新租赁。
/// 使用 `tokio::runtime::Handle::block_on` 桥接同步 trait 到异步 API。
pub struct LeasedKeySource {
    client: Arc<XiaozhuClient>,
    model: String,
    cache: Mutex<Option<LeasedKey>>,
}

impl LeasedKeySource {
    pub fn new(client: Arc<XiaozhuClient>, model: impl Into<String>) -> Self {
        Self {
            client,
            model: model.into(),
            cache: Mutex::new(None),
        }
    }

    fn block_on_lease(&self) -> Result<LeasedKey, ProviderError> {
        tokio::runtime::Handle::try_current()
            .map_err(|_| {
                ProviderError::KeySource(
                    "no tokio runtime available for key lease".to_string(),
                )
            })
            .and_then(|handle| {
                handle.block_on(async {
                    self.client
                        .key
                        .lease_key(&self.model)
                        .await
                        .map_err(map_auth_error)
                })
            })
    }
}

impl KeySource for LeasedKeySource {
    fn get_api_key(&self) -> Result<String, ProviderError> {
        let cached = self.cache.blocking_lock();
        if let Some(ref key) = *cached {
            if !key.is_expired() {
                return Ok(key.api_key.clone());
            }
        }
        drop(cached);

        let key = self.block_on_lease()?;
        let api_key = key.api_key.clone();

        let mut cache = self.cache.blocking_lock();
        *cache = Some(key);

        Ok(api_key)
    }
}

fn map_auth_error(err: xz_auth_core::AuthError) -> ProviderError {
    match &err {
        xz_auth_core::AuthError::InvalidToken
        | xz_auth_core::AuthError::TokenExpired
        | xz_auth_core::AuthError::MissingToken
        | xz_auth_core::AuthError::InvalidRefreshToken => {
            ProviderError::Auth(err.to_string())
        }
        xz_auth_core::AuthError::NetworkError(msg) => ProviderError::Network {
            message: msg.clone(),
            detail: None,
        },
        _ => ProviderError::KeySource(err.to_string()),
    }
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
