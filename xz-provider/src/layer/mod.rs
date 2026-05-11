mod retry;

pub use retry::{RetryLayer, RetryLayerService, TelemetryLayer, TelemetryLayerService};

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;

use crate::error::ProviderError;
use crate::traits::LlmProvider;
use crate::types::{CompletionRequest, CompletionResponse, ModelInfo, RequestOptions, StreamEvent};

/// Provider 中间件 Layer —— 包装 LlmProvider，可以在调用前后插入逻辑
///
/// 设计为可堆叠的：每个 Layer 接收内层 LlmProvider，返回包装后的 LlmProvider。
/// 通过泛型实现真正的链式调用，不需要 trait object。
pub trait ProviderLayer<S: LlmProvider> {
    type Stack: LlmProvider;

    fn wrap(self, inner: S) -> Self::Stack;
}

/// 链式包装后的 Provider 实现
pub struct Layered<L, S> {
    layer: L,
    inner: S,
}

impl<L, S: LlmProvider> Layered<L, S> {
    pub fn new(layer: L, inner: S) -> Self {
        Self { layer, inner }
    }
}

impl<L, S: LlmProvider> std::fmt::Debug for Layered<L, S>
where
    S: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Layered")
            .field("inner", &self.inner)
            .finish()
    }
}

#[async_trait]
impl<L, S: LlmProvider> LlmProvider for Layered<L, S>
where
    L: LayerService<S> + Send + Sync,
{
    async fn complete(
        &self,
        request: CompletionRequest,
        options: RequestOptions,
    ) -> Result<CompletionResponse, ProviderError> {
        self.layer.complete(&self.inner, request, options).await
    }

    async fn complete_stream(
        &self,
        request: CompletionRequest,
        options: RequestOptions,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send>>,
        ProviderError,
    > {
        self.layer.complete_stream(&self.inner, request, options).await
    }

    fn models(&self) -> &[ModelInfo] {
        self.inner.models()
    }

    fn name(&self) -> &str {
        self.inner.name()
    }
}

/// 每个 Layer 需要实现的服务 trait（处理请求逻辑）
#[async_trait]
pub trait LayerService<S: LlmProvider>: Send + Sync {
    async fn complete(
        &self,
        inner: &S,
        request: CompletionRequest,
        options: RequestOptions,
    ) -> Result<CompletionResponse, ProviderError>;

    async fn complete_stream(
        &self,
        inner: &S,
        request: CompletionRequest,
        options: RequestOptions,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send>>,
        ProviderError,
    >;
}
