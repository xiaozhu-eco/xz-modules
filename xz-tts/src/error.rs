use std::time::Duration;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum XzTtsError {
    #[error("authentication failed: {message}")]
    Auth { message: String },
    #[error("network error: {message}")]
    Network { message: String },
    #[error("rate limited{}", retry_after.map(|d| format!(" (retry after {:?})", d)).unwrap_or_default())]
    RateLimit { retry_after: Option<Duration> },
    #[error("timeout: {message}")]
    Timeout { message: String },
    #[error("configuration error: {message}")]
    Config { message: String },
    #[error("voice not found: {voice_id}")]
    VoiceNotFound { voice_id: String },
    #[error("text too long: {len} chars (max {max})")]
    TextTooLong { len: usize, max: usize },
    #[error("format error: {message}")]
    Format { message: String },
    #[error("protocol error {code}: {message}")]
    Protocol { code: i32, message: String },
    #[error("internal error: {message}")]
    Internal { message: String },
}

impl XzTtsError {
    /// Following xz-provider's ProviderError::is_retryable() semantics:
    /// - Network, RateLimit, Timeout → retryable (true)
    /// - Auth, Config, VoiceNotFound, TextTooLong, Format, Protocol, Internal → NOT retryable (false)
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Network { .. } | Self::RateLimit { .. } | Self::Timeout { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retryable_errors_return_true() {
        assert!(XzTtsError::Network { message: "dns failure".into() }.is_retryable());
        assert!(XzTtsError::RateLimit { retry_after: Some(Duration::from_secs(5)) }.is_retryable());
        assert!(XzTtsError::Timeout { message: "request timed out".into() }.is_retryable());
    }

    #[test]
    fn non_retryable_errors_return_false() {
        assert!(!XzTtsError::Auth { message: "bad token".into() }.is_retryable());
        assert!(!XzTtsError::Config { message: "missing api key".into() }.is_retryable());
        assert!(!XzTtsError::VoiceNotFound { voice_id: "en-US-1".into() }.is_retryable());
        assert!(!XzTtsError::TextTooLong { len: 5000, max: 3000 }.is_retryable());
        assert!(!XzTtsError::Format { message: "invalid json".into() }.is_retryable());
        assert!(!XzTtsError::Protocol { code: 400, message: "bad request".into() }.is_retryable());
        assert!(!XzTtsError::Internal { message: "panic".into() }.is_retryable());
    }

    #[test]
    fn display_messages_are_human_readable() {
        assert_eq!(
            XzTtsError::Auth { message: "bad token".into() }.to_string(),
            "authentication failed: bad token"
        );
        assert_eq!(
            XzTtsError::Network { message: "connection reset".into() }.to_string(),
            "network error: connection reset"
        );
        assert_eq!(
            XzTtsError::RateLimit { retry_after: None }.to_string(),
            "rate limited"
        );
        assert_eq!(
            XzTtsError::RateLimit { retry_after: Some(Duration::from_secs(5)) }.to_string(),
            "rate limited (retry after 5s)"
        );
        assert_eq!(
            XzTtsError::Timeout { message: "deadline exceeded".into() }.to_string(),
            "timeout: deadline exceeded"
        );
        assert_eq!(
            XzTtsError::Config { message: "missing endpoint".into() }.to_string(),
            "configuration error: missing endpoint"
        );
        assert_eq!(
            XzTtsError::VoiceNotFound { voice_id: "voice-1".into() }.to_string(),
            "voice not found: voice-1"
        );
        assert_eq!(
            XzTtsError::TextTooLong { len: 42, max: 40 }.to_string(),
            "text too long: 42 chars (max 40)"
        );
        assert_eq!(
            XzTtsError::Format { message: "bad response".into() }.to_string(),
            "format error: bad response"
        );
        assert_eq!(
            XzTtsError::Protocol { code: 418, message: "teapot".into() }.to_string(),
            "protocol error 418: teapot"
        );
        assert_eq!(
            XzTtsError::Internal { message: "oops".into() }.to_string(),
            "internal error: oops"
        );
    }
}
