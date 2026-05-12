use std::time::Duration;

use crate::types::TokenUsage;

/// 记录 LLM 补全事件
pub fn emit_completion_event(
    provider: &str,
    model: &str,
    prompt_tokens: u32,
    completion_tokens: u32,
    latency: Duration,
    success: bool,
) {
    let latency_ms = latency.as_millis() as u64;
    tracing::info!(
        target: "xz_provider",
        provider = %provider,
        model = %model,
        prompt_tokens = prompt_tokens,
        completion_tokens = completion_tokens,
        latency_ms = latency_ms,
        success = success,
        "llm_completion"
    );
}

/// 记录错误事件
pub fn emit_error_event(provider: &str, model: &str, error: &crate::error::ProviderError) {
    tracing::error!(
        target: "xz_provider",
        provider = %provider,
        model = %model,
        error = %error,
        "llm_error"
    );
}

/// Token 用量统计
pub struct TokenStats {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_requests: u64,
}

impl TokenStats {
    pub fn new() -> Self {
        Self {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_requests: 0,
        }
    }

    pub fn record(&mut self, usage: &TokenUsage) {
        self.prompt_tokens += usage.prompt_tokens as u64;
        self.completion_tokens += usage.completion_tokens as u64;
        self.total_requests += 1;
    }
}

impl Default for TokenStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_stats_new() {
        let stats = TokenStats::new();
        assert_eq!(stats.prompt_tokens, 0);
        assert_eq!(stats.completion_tokens, 0);
        assert_eq!(stats.total_requests, 0);
    }

    #[test]
    fn test_token_stats_default() {
        let stats = TokenStats::default();
        assert_eq!(stats.prompt_tokens, 0);
        assert_eq!(stats.completion_tokens, 0);
        assert_eq!(stats.total_requests, 0);
    }

    #[test]
    fn test_token_stats_record() {
        let mut stats = TokenStats::new();
        stats.record(&TokenUsage {
            prompt_tokens: 10,
            completion_tokens: 20,
            ..Default::default()
        });
        assert_eq!(stats.prompt_tokens, 10);
        assert_eq!(stats.completion_tokens, 20);
        assert_eq!(stats.total_requests, 1);
    }

    #[test]
    fn test_token_stats_record_cumulative() {
        let mut stats = TokenStats::new();
        for _ in 0..3 {
            stats.record(&TokenUsage {
                prompt_tokens: 100,
                completion_tokens: 50,
                ..Default::default()
            });
        }
        assert_eq!(stats.prompt_tokens, 300);
        assert_eq!(stats.completion_tokens, 150);
        assert_eq!(stats.total_requests, 3);
    }
}
