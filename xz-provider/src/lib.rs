//! # xz-provider
//!
//! LLM 服务提供者抽象层 — 所有依赖 LLM 调用的 crate 的最底层基础设施。
//!
//! ## 核心 Trait
//!
//! - [`LlmProvider`]: 统一的 LLM 服务提供者接口
//!
//! ## 内置实现
//!
//! - `OpenAiProvider` — OpenAI 兼容 API（OpenAI、DeepSeek、通义千问等）
//! - `ClaudeProvider` — Anthropic Claude API
//! - `LocalProvider`  — 本地模型（Ollama / llama.cpp）
//!
//! ## 快速开始
//!
//! ```rust,no_run
//! use xz_provider::{ProviderBuilder, ProviderConfig, LlmProvider};
//!
//! # async fn example() {
//! let router = ProviderBuilder::new()
//!     .with_config(ProviderConfig::from_json(r#"{
//!         "default_model": "gpt-4o",
//!         "providers": {
//!             "openai": {
//!                 "provider_type": "open_ai",
//!                 "api_key": "sk-xxx",
//!                 "models": [{"name": "gpt-4o", "capabilities": {"context_window": 128000, "max_output_tokens": 4096}}]
//!             }
//!         },
//!         "routing": {}
//!     }"#).unwrap())
//!     .build().await.unwrap();
//!
//! let resp = router.complete(
//!     &xz_provider::RouteContext::default(),
//!     xz_provider::CompletionRequest::new("gpt-4o", vec![
//!         xz_provider::Message::user("Hello!"),
//!     ]),
//!     xz_provider::RequestOptions::default(),
//! ).await.unwrap();
//! # }
//! ```

pub mod accumulator;
pub mod builder;
pub mod cancel;
pub mod config;
pub mod error;
pub mod key_source;
pub mod layer;
pub mod observability;
pub mod providers;
pub mod router;
pub mod traits;
pub mod types;

pub use accumulator::ToolCallAccumulator;
pub use builder::ProviderBuilder;
pub use cancel::CancellationToken;
pub use config::{
    ConfigWatcher, FallbackCondition as ConfigFallbackCondition,
    FallbackEntry as ConfigFallbackEntry, ModelConfig, ProviderConfig,
    ProviderDefinition, ProviderType, RouteRule,
};
pub use error::{ProviderError, RetryStrategy};
pub use key_source::KeySource;
pub use layer::{
    Layered as ProviderLayered, LayerService, ProviderLayer, RetryLayer, TelemetryLayer,
};
pub use router::{
    CostPreference, FallbackCondition, FallbackEntry, HealthState, LatencyTracker,
    ProviderRouter, RouteContext, RouteDecision,
};
pub use traits::LlmProvider;
pub use types::{
    CacheControl, CacheInfo, CapabilityRequest, CompletionRequest, CompletionResponse,
    ContentPart, FinishReason, ImageDetail, Message, MessageContent,
    ModelCapabilities, ModelInfo, ModelLimits, ModelPricing, Modality,
    ReasoningEffort, RequestOptions, ResponseFormat, StreamEvent, ToolCall,
    ToolChoice, ToolDefinition, ToolResult, TokenUsage,
};
