use std::time::{Duration, SystemTime, UNIX_EPOCH};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum NotifError {
    #[error("channel not found: {0}")]
    ChannelNotFound(String),
    #[error("channel unavailable ({channel}): {reason}")]
    ChannelUnavailable { channel: String, reason: String },
    #[error("rate limited on {channel}, retry after {retry_after_ms}ms")]
    RateLimited { channel: String, retry_after_ms: u64 },
    #[error("template error: {0}")]
    TemplateError(String),
    #[error("preference error: {0}")]
    PreferenceError(String),
    #[error("delivery timeout on {channel}")]
    DeliveryTimeout { channel: String },
    #[error("all channels failed: {0:?}")]
    AllChannelsFailed(Vec<ChannelError>),
    #[error("do not disturb until {until:?}")]
    DoNotDisturb { until: SystemTime },
    #[error("internal notification error: {0}")]
    Internal(Box<dyn std::error::Error + Send + Sync>),
}

#[derive(Debug, Error)]
pub enum ChannelError {
    #[error("connection error: {0}")]
    Connection(String),
    #[error("auth error: {0}")]
    Auth(String),
    #[error("rate limited, retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },
    #[error("invalid payload: {0}")]
    InvalidPayload(String),
    #[error("device not registered: {0}")]
    DeviceNotRegistered(String),
    #[error("timeout")]
    Timeout,
}

impl NotifError {
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::ChannelUnavailable { .. }
            | Self::RateLimited { .. }
            | Self::DeliveryTimeout { .. } => true,
            Self::AllChannelsFailed(errors) => errors.iter().any(|e| {
                matches!(e, ChannelError::Connection(_) | ChannelError::RateLimited { .. } | ChannelError::Timeout)
            }),
            _ => false,
        }
    }

    pub fn is_cancelled(&self) -> bool {
        matches!(self, Self::DoNotDisturb { .. })
    }

    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::RateLimited { retry_after_ms, .. } => Some(Duration::from_millis(*retry_after_ms)),
            Self::DeliveryTimeout { .. } => Some(Duration::from_secs(5)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RetryStrategy {
    pub max_retries: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub jitter: bool,
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self { max_retries: 3, base_delay: Duration::from_secs(1), max_delay: Duration::from_secs(30), jitter: true }
    }
}

impl RetryStrategy {
    pub fn new(max_retries: u32, base_delay: Duration, max_delay: Duration, jitter: bool) -> Self {
        Self { max_retries, base_delay, max_delay, jitter }
    }

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
    (SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as f64
        / 1_000_000_000.0)
        .fract()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retryable_channel_unavailable() {
        assert!(NotifError::ChannelUnavailable { channel: "sms".into(), reason: "offline".into() }.is_retryable());
    }

    #[test]
    fn retryable_rate_limited() {
        assert!(NotifError::RateLimited { channel: "sms".into(), retry_after_ms: 1000 }.is_retryable());
    }

    #[test]
    fn retryable_delivery_timeout() {
        assert!(NotifError::DeliveryTimeout { channel: "push".into() }.is_retryable());
    }

    #[test]
    fn retryable_all_channels_failed_with_retryable_inner_error() {
        assert!(NotifError::AllChannelsFailed(vec![ChannelError::Timeout]).is_retryable());
    }

    #[test]
    fn not_retryable_channel_not_found() {
        assert!(!NotifError::ChannelNotFound("sms".into()).is_retryable());
    }

    #[test]
    fn not_retryable_template_error() {
        assert!(!NotifError::TemplateError("missing token".into()).is_retryable());
    }

    #[test]
    fn not_retryable_preference_error() {
        assert!(!NotifError::PreferenceError("muted".into()).is_retryable());
    }

    #[test]
    fn not_retryable_do_not_disturb() {
        assert!(!NotifError::DoNotDisturb { until: SystemTime::now() }.is_retryable());
    }

    #[test]
    fn not_retryable_all_channels_failed_with_fatal_inner_error() {
        assert!(!NotifError::AllChannelsFailed(vec![ChannelError::InvalidPayload("bad".into())]).is_retryable());
    }

    #[test]
    fn not_retryable_internal() {
        let e: Box<dyn std::error::Error + Send + Sync> = Box::new(std::io::Error::other("boom"));
        assert!(!NotifError::Internal(e).is_retryable());
    }

    #[test]
    fn cancel_status_only_for_dnd() {
        assert!(NotifError::DoNotDisturb { until: SystemTime::now() }.is_cancelled());
        assert!(!NotifError::ChannelNotFound("sms".into()).is_cancelled());
    }

    #[test]
    fn retry_after_rate_limited() {
        assert_eq!(NotifError::RateLimited { channel: "sms".into(), retry_after_ms: 2500 }.retry_after(), Some(Duration::from_millis(2500)));
    }

    #[test]
    fn retry_after_delivery_timeout() {
        assert_eq!(NotifError::DeliveryTimeout { channel: "push".into() }.retry_after(), Some(Duration::from_secs(5)));
    }

    #[test]
    fn retry_after_other_variants_none() {
        assert!(NotifError::ChannelNotFound("x".into()).retry_after().is_none());
        assert!(NotifError::TemplateError("x".into()).retry_after().is_none());
    }

    #[test]
    fn retry_strategy_default() {
        let s = RetryStrategy::default();
        assert_eq!(s.max_retries, 3);
        assert_eq!(s.base_delay, Duration::from_secs(1));
        assert_eq!(s.max_delay, Duration::from_secs(30));
        assert!(s.jitter);
    }

    #[test]
    fn retry_strategy_exponential_no_jitter() {
        let s = RetryStrategy::new(5, Duration::from_secs(1), Duration::from_secs(30), false);
        assert_eq!(s.delay_for_attempt(0), Duration::from_secs(1));
        assert_eq!(s.delay_for_attempt(1), Duration::from_secs(2));
        assert_eq!(s.delay_for_attempt(2), Duration::from_secs(4));
    }

    #[test]
    fn retry_strategy_max_delay_cap() {
        let s = RetryStrategy::new(10, Duration::from_secs(10), Duration::from_secs(30), false);
        assert_eq!(s.delay_for_attempt(4), Duration::from_secs(30));
    }

    #[test]
    fn retry_strategy_jitter_bounds() {
        let s = RetryStrategy::new(3, Duration::from_millis(100), Duration::from_secs(10), true);
        for attempt in 0..5 {
            let delay = s.delay_for_attempt(attempt);
            let expected_max = (100.0 * 2u64.pow(attempt.min(30)) as f64).min(10_000.0);
            let ms = delay.as_millis() as f64;
            assert!(ms >= expected_max * 0.4, "attempt={} ms={} min={}", attempt, ms, expected_max * 0.4);
            assert!(ms <= expected_max * 1.1, "attempt={} ms={} max={}", attempt, ms, expected_max * 1.1);
        }
    }

    #[test]
    fn display_strings_include_fields() {
        let err = NotifError::RateLimited { channel: "sms".into(), retry_after_ms: 5000 };
        assert!(format!("{}", err).contains("5000"));
        assert!(format!("{}", ChannelError::Connection("refused".into())).contains("refused"));
    }
}
