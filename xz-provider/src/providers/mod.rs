#[cfg(feature = "openai")]
mod openai;
#[cfg(feature = "claude")]
mod claude;
mod local;
mod sse;

#[cfg(feature = "openai")]
pub use openai::OpenAiProvider;
#[cfg(feature = "claude")]
pub use claude::ClaudeProvider;
pub use local::LocalProvider;

/// Parse Retry-After header value from an HTTP response.
/// Supports integer seconds format ("120") and float ("120.5").
/// Falls back to `default_ms` if header is missing or unparseable.
pub(crate) fn parse_retry_after(resp: &reqwest::Response, default_ms: u64) -> u64 {
    resp.headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim().parse::<f64>().ok())
        .map(|secs| (secs * 1000.0) as u64)
        .unwrap_or(default_ms)
}
