use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;

use super::{LayerService, ProviderLayer};
use crate::error::{ProviderError, RetryStrategy};
use crate::traits::LlmProvider;
use crate::types::{CompletionRequest, CompletionResponse, RequestOptions, StreamEvent};

/// 重试中间件 —— 指数退避 + 抖动，只重试 Transient 错误
pub struct RetryLayer {
    strategy: RetryStrategy,
}

impl RetryLayer {
    pub fn new(strategy: RetryStrategy) -> Self {
        Self { strategy }
    }
}

impl<S: LlmProvider + 'static> ProviderLayer<S> for RetryLayer {
    type Stack = super::Layered<RetryLayerService, S>;

    fn wrap(self, inner: S) -> Self::Stack {
        super::Layered::new(
            RetryLayerService {
                strategy: self.strategy,
            },
            inner,
        )
    }
}

pub struct RetryLayerService {
    strategy: RetryStrategy,
}

#[async_trait]
impl<S: LlmProvider> LayerService<S> for RetryLayerService {
    async fn complete(
        &self,
        inner: &S,
        request: CompletionRequest,
        options: RequestOptions,
    ) -> Result<CompletionResponse, ProviderError> {
        let mut attempt = 0u32;
        loop {
            match inner.complete(request.clone(), options.clone()).await {
                Ok(resp) => return Ok(resp),
                Err(e) if e.is_retryable() && attempt < self.strategy.max_retries => {
                    let delay = self.strategy.delay_for_attempt(attempt);
                    tokio::time::sleep(delay).await;
                    attempt += 1;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

    async fn complete_stream(
        &self,
        inner: &S,
        request: CompletionRequest,
        options: RequestOptions,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send>>,
        ProviderError,
    > {
        // Stream retry: only retry the initial connection, not the stream itself
        let mut attempt = 0u32;
        loop {
            match inner.complete_stream(request.clone(), options.clone()).await {
                Ok(stream) => return Ok(stream),
                Err(e) if e.is_retryable() && attempt < self.strategy.max_retries => {
                    let delay = self.strategy.delay_for_attempt(attempt);
                    tokio::time::sleep(delay).await;
                    attempt += 1;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }
}

impl Default for RetryLayer {
    fn default() -> Self {
        Self::new(RetryStrategy::default())
    }
}

/// 日志/指标中间件 —— 自动记录请求耗时、token 用量、错误
pub struct TelemetryLayer;

impl<S: LlmProvider + 'static> ProviderLayer<S> for TelemetryLayer {
    type Stack = super::Layered<TelemetryLayerService, S>;

    fn wrap(self, inner: S) -> Self::Stack {
        super::Layered::new(TelemetryLayerService, inner)
    }
}

pub struct TelemetryLayerService;

#[async_trait]
impl<S: LlmProvider> LayerService<S> for TelemetryLayerService {
    async fn complete(
        &self,
        inner: &S,
        request: CompletionRequest,
        options: RequestOptions,
    ) -> Result<CompletionResponse, ProviderError> {
        let start = std::time::Instant::now();
        let model = request.model.clone().unwrap_or_default();
        let has_tools = request.tools.is_some();
        let tool_count = request.tools.as_ref().map(|t| t.len()).unwrap_or(0);

        let _span = tracing::info_span!(
            "llm_completion",
            provider = %inner.name(),
            model = %model,
            has_tools = has_tools,
            tool_count = tool_count,
            stream = false,
        );

        let result = inner.complete(request, options).await;
        let latency = start.elapsed();

        match &result {
            Ok(resp) => {
                tracing::info!(
                    target: "xz_provider",
                    prompt_tokens = resp.usage.prompt_tokens,
                    completion_tokens = resp.usage.completion_tokens,
                    cached_tokens = resp.usage.cached_tokens.unwrap_or(0),
                    latency_ms = latency.as_millis() as u64,
                    finish_reason = ?resp.finish_reason,
                    tool_calls = resp.tool_calls.len(),
                    "llm_completion_completed"
                );
            }
            Err(e) => {
                tracing::warn!(
                    target: "xz_provider",
                    error = %e,
                    is_retryable = e.is_retryable(),
                    "llm_completion_failed"
                );
            }
        }

        result
    }

    async fn complete_stream(
        &self,
        inner: &S,
        request: CompletionRequest,
        options: RequestOptions,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send>>,
        ProviderError,
    > {
        inner.complete_stream(request, options).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use std::time::Duration;

    #[derive(Debug)]
    struct MockProvider {
        name: String,
        behavior: std::sync::Mutex<Vec<Result<CompletionResponse, ProviderError>>>,
    }

    #[async_trait]
    impl LlmProvider for MockProvider {
        fn name(&self) -> &str {
            &self.name
        }
        fn models(&self) -> &[ModelInfo] {
            &[]
        }
        async fn complete(
            &self,
            _request: CompletionRequest,
            _options: RequestOptions,
        ) -> Result<CompletionResponse, ProviderError> {
            let mut b = self.behavior.lock().unwrap();
            if b.is_empty() {
                Ok(CompletionResponse {
                    content: Some("ok".into()),
                    thinking: None,
                    tool_calls: vec![],
                    usage: TokenUsage::new(0, 0),
                    model: "mock".into(),
                    finish_reason: FinishReason::Stop,
                    latency_ms: 0,
                    cache_info: None,
                })
            } else {
                b.remove(0)
            }
        }
        async fn complete_stream(
            &self,
            _request: CompletionRequest,
            _options: RequestOptions,
        ) -> Result<
            Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send>>,
            ProviderError,
        > {
            Err(ProviderError::Config("not implemented".into()))
        }
    }

    #[tokio::test]
    async fn test_retry_layer_success() {
        let mock = MockProvider {
            name: "test".into(),
            behavior: std::sync::Mutex::new(vec![]),
        };
        let layer = RetryLayer::new(RetryStrategy {
            max_retries: 2,
            base_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(10),
            jitter: false,
        });
        let stacked = layer.wrap(mock);
        let resp = stacked
            .complete(CompletionRequest::default(), RequestOptions::default())
            .await
            .unwrap();
        assert_eq!(resp.content.as_deref(), Some("ok"));
    }

    #[tokio::test]
    async fn test_retry_layer_retries_then_succeeds() {
        let mock = MockProvider {
            name: "test".into(),
            behavior: std::sync::Mutex::new(vec![
                Err(ProviderError::Overloaded),
                Err(ProviderError::Timeout { timeout_ms: 1000 }),
                Ok(CompletionResponse {
                    content: Some("recovered".into()),
                    thinking: None,
                    tool_calls: vec![],
                    usage: TokenUsage::new(0, 0),
                    model: "mock".into(),
                    finish_reason: FinishReason::Stop,
                    latency_ms: 0,
                    cache_info: None,
                }),
            ]),
        };
        let layer = RetryLayer::new(RetryStrategy {
            max_retries: 3,
            base_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(10),
            jitter: false,
        });
        let stacked = layer.wrap(mock);
        let resp = stacked
            .complete(CompletionRequest::default(), RequestOptions::default())
            .await
            .unwrap();
        assert_eq!(resp.content.as_deref(), Some("recovered"));
    }

    #[tokio::test]
    async fn test_retry_layer_fatal_error_not_retried() {
        let mock = MockProvider {
            name: "test".into(),
            behavior: std::sync::Mutex::new(vec![
                Err(ProviderError::Auth("bad key".into())),
                Ok(CompletionResponse {
                    content: Some("should not reach".into()),
                    thinking: None,
                    tool_calls: vec![],
                    usage: TokenUsage::new(0, 0),
                    model: "mock".into(),
                    finish_reason: FinishReason::Stop,
                    latency_ms: 0,
                    cache_info: None,
                }),
            ]),
        };
        let layer = RetryLayer::new(RetryStrategy {
            max_retries: 3,
            base_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(10),
            jitter: false,
        });
        let stacked = layer.wrap(mock);
        let err = stacked
            .complete(CompletionRequest::default(), RequestOptions::default())
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Auth(_)));
    }
}
