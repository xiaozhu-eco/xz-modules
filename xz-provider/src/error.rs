use std::time::Duration;

use thiserror::Error;

use crate::types::CapabilityRequest;

/// Provider 错误类型
///
/// 分类：
/// - Transient（可重试）：Network, RateLimit, Overloaded, Timeout
/// - Fatal（不可重试）：Cancelled, Config, Auth, TokenLimit, Format, 等
#[derive(Debug, Error)]
pub enum ProviderError {
    // ── 可重试错误（Transient）──
    #[error("网络错误: {message}")]
    Network { message: String, detail: Option<String> },

    #[error("限流，{retry_after_ms}ms 后重试")]
    RateLimit { retry_after_ms: u64 },

    #[error("Provider 过载 (503)，稍后重试")]
    Overloaded,

    #[error("超时 ({timeout_ms}ms)")]
    Timeout { timeout_ms: u64 },

    // ── 不可重试错误（Fatal）──
    #[error("请求被取消")]
    Cancelled,

    #[error("配置错误: {0}")]
    Config(String),

    #[error("认证失败: {0}")]
    Auth(String),

    #[error("Token 超限: prompt={actual}, limit={limit}")]
    TokenLimit { actual: usize, limit: usize },

    #[error("输出格式错误: {0}")]
    Format(String),

    #[error("模型不存在: {0}")]
    ModelNotFound(String),

    #[error("不支持的能力: {model} 不支持 {capability}")]
    UnsupportedCapability { model: String, capability: String },

    #[error("没有模型满足能力要求: {required:?}")]
    NoModelForCapability { required: CapabilityRequest },

    #[error("Provider 错误 [{status}]: {message}")]
    Internal { status: u16, message: String },

    #[error("没有可用的 Provider 路由: {0}")]
    NoRoute(String),

    #[error("无法获取 API Key: {0}")]
    KeySource(String),
}

impl ProviderError {
    /// 判断是否为可重试错误
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ProviderError::Network { .. }
                | ProviderError::RateLimit { .. }
                | ProviderError::Overloaded
                | ProviderError::Timeout { .. }
        )
    }

    /// 判断是否为用户主动取消
    pub fn is_cancelled(&self) -> bool {
        matches!(self, ProviderError::Cancelled)
    }

    /// 建议的重试等待时间
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            ProviderError::RateLimit { retry_after_ms } => {
                Some(Duration::from_millis(*retry_after_ms))
            }
            ProviderError::Overloaded => Some(Duration::from_secs(5)),
            _ => None,
        }
    }
}

/// 重试策略
#[derive(Debug, Clone)]
pub struct RetryStrategy {
    pub max_retries: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub jitter: bool,
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
            jitter: true,
        }
    }
}

impl RetryStrategy {
    pub fn new(max_retries: u32, base_delay: Duration, max_delay: Duration, jitter: bool) -> Self {
        Self {
            max_retries,
            base_delay,
            max_delay,
            jitter,
        }
    }

    /// 计算第 n 次重试的等待时间（指数退避），不包含抖动时基数为 2^attempt
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let base_ms = self.base_delay.as_millis() as f64;
        let delay = base_ms * 2u64.pow(attempt.min(30)) as f64;
        let delay = delay.min(self.max_delay.as_millis() as f64);
        if self.jitter {
            let jitter = rand_dummy();
            Duration::from_millis((delay * (0.5 + jitter * 0.5)) as u64)
        } else {
            Duration::from_millis(delay as u64)
        }
    }
}

fn rand_dummy() -> f64 {
    (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as f64
        / 1_000_000_000.0)
        .fract()
}

impl From<reqwest::Error> for ProviderError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            ProviderError::Timeout {
                timeout_ms: 0,
            }
        } else if e.is_connect() || e.is_request() {
            ProviderError::Network {
                message: e.to_string(),
                detail: None,
            }
        } else if let Some(status) = e.status() {
            if status.as_u16() == 503 {
                ProviderError::Overloaded
            } else if status.as_u16() == 429 {
                ProviderError::RateLimit { retry_after_ms: 0 }
            } else if status.as_u16() == 401 || status.as_u16() == 403 {
                ProviderError::Auth(e.to_string())
            } else {
                ProviderError::Internal {
                    status: status.as_u16(),
                    message: e.to_string(),
                }
            }
        } else {
            ProviderError::Network {
                message: e.to_string(),
                detail: None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_retryable_network() {
        let err = ProviderError::Network { message: "refused".into(), detail: None };
        assert!(err.is_retryable());
        assert!(!err.is_cancelled());
    }

    #[test]
    fn test_is_retryable_rate_limit() {
        let err = ProviderError::RateLimit { retry_after_ms: 5000 };
        assert!(err.is_retryable());
    }

    #[test]
    fn test_is_retryable_overloaded() {
        let err = ProviderError::Overloaded;
        assert!(err.is_retryable());
    }

    #[test]
    fn test_is_retryable_timeout() {
        let err = ProviderError::Timeout { timeout_ms: 30000 };
        assert!(err.is_retryable());
    }

    #[test]
    fn test_is_not_retryable() {
        assert!(!ProviderError::Cancelled.is_retryable());
        assert!(!ProviderError::Config("bad".into()).is_retryable());
        assert!(!ProviderError::Auth("denied".into()).is_retryable());
        assert!(!ProviderError::TokenLimit { actual: 100, limit: 50 }.is_retryable());
        assert!(!ProviderError::Format("bad json".into()).is_retryable());
        assert!(!ProviderError::ModelNotFound("gpt-3".into()).is_retryable());
        assert!(!ProviderError::UnsupportedCapability { model: "gpt".into(), capability: "vision".into() }.is_retryable());
        assert!(!ProviderError::NoModelForCapability { required: CapabilityRequest::default() }.is_retryable());
        assert!(!ProviderError::Internal { status: 500, message: "error".into() }.is_retryable());
        assert!(!ProviderError::NoRoute("nowhere".into()).is_retryable());
    }

    #[test]
    fn test_is_cancelled() {
        assert!(ProviderError::Cancelled.is_cancelled());
        assert!(!ProviderError::Config("test".into()).is_cancelled());
        assert!(!ProviderError::Network { message: "x".into(), detail: None }.is_cancelled());
    }

    #[test]
    fn test_retry_after_rate_limit() {
        let err = ProviderError::RateLimit { retry_after_ms: 2000 };
        assert_eq!(err.retry_after(), Some(Duration::from_millis(2000)));
    }

    #[test]
    fn test_retry_after_overloaded() {
        let err = ProviderError::Overloaded;
        assert_eq!(err.retry_after(), Some(Duration::from_secs(5)));
    }

    #[test]
    fn test_retry_after_fatal_errors() {
        let fatal_errors = vec![
            ProviderError::Network { message: "x".into(), detail: None },
            ProviderError::Timeout { timeout_ms: 1000 },
            ProviderError::Cancelled,
            ProviderError::Config("x".into()),
            ProviderError::Auth("x".into()),
        ];
        for err in fatal_errors {
            assert!(err.retry_after().is_none(), "{:?} should not have retry_after", err);
        }
    }

    #[test]
    fn test_error_display_network() {
        let err = ProviderError::Network { message: "refused".into(), detail: None };
        let msg = format!("{}", err);
        assert!(msg.contains("refused"));
    }

    #[test]
    fn test_error_display_rate_limit() {
        let err = ProviderError::RateLimit { retry_after_ms: 5000 };
        let msg = format!("{}", err);
        assert!(msg.contains("5000"));
    }

    #[test]
    fn test_retry_strategy_default() {
        let s = RetryStrategy::default();
        assert_eq!(s.max_retries, 3);
        assert_eq!(s.base_delay, Duration::from_secs(1));
        assert_eq!(s.max_delay, Duration::from_secs(30));
        assert!(s.jitter);
    }

    #[test]
    fn test_retry_strategy_exponential_no_jitter() {
        let s = RetryStrategy::new(5, Duration::from_millis(100), Duration::from_secs(10), false);
        assert_eq!(s.delay_for_attempt(0), Duration::from_millis(100));
        assert_eq!(s.delay_for_attempt(1), Duration::from_millis(200));
        assert_eq!(s.delay_for_attempt(2), Duration::from_millis(400));
        assert_eq!(s.delay_for_attempt(3), Duration::from_millis(800));
    }

    #[test]
    fn test_retry_strategy_max_delay_cap() {
        let s = RetryStrategy::new(10, Duration::from_secs(1), Duration::from_millis(500), false);
        // 2^10 * 1000ms exceeds 500ms cap
        assert_eq!(s.delay_for_attempt(10), Duration::from_millis(500));
    }

    #[test]
    fn test_retry_strategy_jitter_within_bounds() {
        let s = RetryStrategy::new(3, Duration::from_millis(100), Duration::from_secs(10), true);
        for attempt in 0..5 {
            let delay = s.delay_for_attempt(attempt);
            let max_delay = (100.0 * 2u64.pow(attempt.min(30)) as f64).min(10_000.0);
            let ms = delay.as_millis() as f64;
            assert!(ms >= max_delay * 0.4, "attempt={} ms={} min={}", attempt, ms, max_delay * 0.4);
            assert!(ms <= max_delay * 1.1, "attempt={} ms={} max={}", attempt, ms, max_delay * 1.1);
        }
    }

    #[test]
    fn test_retry_strategy_display() {
        let s = RetryStrategy::default();
        assert_eq!(s.max_retries, 3);
    }
}
