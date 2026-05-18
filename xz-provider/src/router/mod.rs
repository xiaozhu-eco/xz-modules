mod decision;
pub mod rule;

pub use decision::{
    CostPreference, FallbackCondition, FallbackEntry, LatencyTracker, RouteContext, RouteDecision,
};

use std::collections::HashMap;
use std::pin::Pin;
use tokio::sync::RwLock;

use futures::Stream;

use crate::config::RouteRule as ConfigRouteRule;
use crate::error::ProviderError;
use crate::traits::LlmProvider;
use crate::types::{
    CompletionRequest, CompletionResponse, ModelInfo, RequestOptions, StreamEvent,
};

/// 路由器 —— 根据能力需求、成本策略、回退链选择最优模型
///
/// **不实现 LlmProvider**。Router 是编排层，负责：
/// 1. 根据路由上下文选择 provider + model
/// 2. 通过中间件栈调用实际 provider
/// 3. 暴露健康状态供上层查询
pub struct ProviderRouter {
    providers: HashMap<String, Box<dyn LlmProvider>>,
    model_registry: Vec<ModelInfo>,
    routing_rules: HashMap<String, ConfigRouteRule>,
    default_model: String,
    latency_tracker: RwLock<LatencyTracker>,
    health_states: HashMap<String, HealthState>,
    key_source: Option<std::sync::Arc<dyn crate::key_source::KeySource>>,
}

/// Provider 健康状态
#[derive(Debug, Clone)]
pub enum HealthState {
    Healthy,
    Degraded { failure_count: u32 },
    CircuitOpen { until: std::time::Instant },
}

impl ProviderRouter {
    pub fn new(
        providers: HashMap<String, Box<dyn LlmProvider>>,
        model_registry: Vec<ModelInfo>,
        routing_rules: HashMap<String, ConfigRouteRule>,
        default_model: String,
        key_source: Option<std::sync::Arc<dyn crate::key_source::KeySource>>,
    ) -> Self {
        let health_states = providers
            .keys()
            .map(|k| (k.clone(), HealthState::Healthy))
            .collect();

        Self {
            providers,
            model_registry,
            routing_rules,
            default_model,
            latency_tracker: RwLock::new(LatencyTracker::default()),
            health_states,
            key_source,
        }
    }

    pub async fn resolve_api_key(&self, _model: &str) -> Option<Result<String, ProviderError>> {
        if let Some(ref source) = self.key_source {
            Some(source.get_api_key().await)
        } else {
            None
        }
    }

    /// 核心路由 + 调用
    pub async fn complete(
        &self,
        ctx: &RouteContext,
        request: CompletionRequest,
        options: RequestOptions,
    ) -> Result<CompletionResponse, ProviderError> {
        let decision = self.resolve(ctx)?;
        let _provider = self.providers.get(&decision.provider).ok_or_else(|| {
            ProviderError::Config(format!("Provider {} not found", decision.provider))
        })?;

        let mut last_error = None;
        for (provider_name, model_name) in decision.iter_entries() {
            let p = self.providers.get(provider_name);
            if let Some(p) = p {
                let mut req = request.clone();
                req.model = req.model.or_else(|| Some(model_name.to_owned()));
                match p.complete(req, options.clone()).await {
                    Ok(resp) => {
                        self.latency_tracker
                            .write()
                            .await
                            .record(model_name, resp.latency_ms);
                        return Ok(resp);
                    }
                    Err(e) if e.is_retryable() => {
                        self.latency_tracker
                            .write()
                            .await
                            .record_error(model_name);
                        last_error = Some(e);
                        continue;
                    }
                    Err(e) => return Err(e),
                }
            }
        }
        Err(last_error.unwrap_or(ProviderError::Config("No provider available".into())))
    }

    /// 流式版本
    pub async fn complete_stream(
        &self,
        ctx: &RouteContext,
        request: CompletionRequest,
        options: RequestOptions,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send>>,
        ProviderError,
    > {
        let decision = self.resolve(ctx)?;
        let provider = self.providers.get(&decision.provider).ok_or_else(|| {
            ProviderError::Config(format!("Provider {} not found", decision.provider))
        })?;

        let mut req = request;
        req.model = req.model.or_else(|| Some(decision.model.clone()));
        provider.complete_stream(req, options).await
    }

    /// 解析路由决策
    pub fn resolve(&self, ctx: &RouteContext) -> Result<RouteDecision, ProviderError> {
        // 1. 指定模型 → 直接使用
        if let Some(ref model) = ctx.model {
            return self.resolve_by_model(model);
        }

        // 2. 命名路由 → 匹配预定义规则（含回退链）
        if let Some(ref name) = ctx.named_route {
            if let Some(rule) = self.routing_rules.iter().find(|(n, _)| *n == name) {
                return self.resolve_rule(rule.0, rule.1);
            }
        }

        // 3. 能力感知路由
        if let Some(ref cap_req) = ctx.capabilities {
            let candidates: Vec<&ModelInfo> = self
                .model_registry
                .iter()
                .filter(|m| m.satisfies(cap_req))
                .collect();

            if candidates.is_empty() {
                return Err(ProviderError::NoModelForCapability {
                    required: cap_req.clone(),
                });
            }

            let healthy: Vec<_> = candidates
                .iter()
                .filter(|m| {
                    m.provider
                        .as_ref()
                        .map(|p| self.is_healthy(p))
                        .unwrap_or(true)
                })
                .copied()
                .collect();

            let pool: &[&ModelInfo] = if healthy.is_empty() {
                &candidates
            } else {
                &healthy
            };

            let selected = match ctx.cost_preference {
                CostPreference::Cheapest => pool
                    .iter()
                    .min_by(|a, b| {
                        a.pricing
                            .input_per_million
                            .partial_cmp(&b.pricing.input_per_million)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|m| m.name.clone()),
                CostPreference::Fastest => self.latency_tracker.blocking_read().fastest(pool),
                _ => pool.first().map(|m| m.name.clone()),
            };

            if let Some(ref model_name) = selected {
                return self.resolve_by_model(model_name);
            }
        }

        // 4. 兜底
        self.resolve_by_model(&self.default_model)
    }

    fn resolve_by_model(&self, model: &str) -> Result<RouteDecision, ProviderError> {
        for (name, provider) in &self.providers {
            for m in provider.models() {
                if m.name == model {
                    return Ok(RouteDecision {
                        model: model.to_owned(),
                        provider: name.clone(),
                        fallback_chain: Vec::new(),
                    });
                }
            }
        }

        // 从 model_registry 中查找
        for m in &self.model_registry {
            if m.name == model {
                if let Some(ref provider) = m.provider {
                    if self.providers.contains_key(provider) {
                        return Ok(RouteDecision {
                            model: model.to_owned(),
                            provider: provider.clone(),
                            fallback_chain: Vec::new(),
                        });
                    }
                }
            }
        }

        Err(ProviderError::ModelNotFound(model.to_owned()))
    }

    fn resolve_rule(
        &self,
        _name: &str,
        rule: &ConfigRouteRule,
    ) -> Result<RouteDecision, ProviderError> {
        let provider_name = rule
            .provider
            .clone()
            .or_else(|| {
                self.providers
                    .iter()
                    .find(|(_, p)| p.models().iter().any(|m| m.name == rule.model))
                    .map(|(n, _)| n.clone())
            })
            .unwrap_or_default();

        let fallback_chain: Vec<FallbackEntry> = rule
            .fallback
            .iter()
            .map(|f| FallbackEntry {
                model: f.model.clone(),
                provider: f
                    .provider
                    .clone()
                    .unwrap_or_else(|| provider_name.clone()),
                condition: match &f.condition {
                    crate::config::FallbackCondition::Always => {
                        FallbackCondition::Always
                    }
                    crate::config::FallbackCondition::RateLimitOnly => {
                        FallbackCondition::RateLimitOnly
                    }
                    crate::config::FallbackCondition::ErrorStatus(codes) => {
                        FallbackCondition::ErrorStatus(codes.clone())
                    }
                },
            })
            .collect();

        Ok(RouteDecision {
            model: rule.model.clone(),
            provider: provider_name,
            fallback_chain,
        })
    }

    pub fn health_states(&self) -> &HashMap<String, HealthState> {
        &self.health_states
    }

    fn is_healthy(&self, provider: &str) -> bool {
        self.health_states
            .get(provider)
            .map(|s| matches!(s, HealthState::Healthy | HealthState::Degraded { .. }))
            .unwrap_or(true)
    }

    pub fn list_providers(&self) -> Vec<&str> {
        self.providers.keys().map(|s| s.as_str()).collect()
    }

    pub fn model_registry(&self) -> &[ModelInfo] {
        &self.model_registry
    }

    pub fn get_routing(&self) -> &HashMap<String, ConfigRouteRule> {
        &self.routing_rules
    }
}

impl std::fmt::Debug for ProviderRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderRouter")
            .field("default_model", &self.default_model)
            .field("models", &self.model_registry.len())
            .field("providers", &self.providers.keys())
            .field("routing", &self.routing_rules)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ProviderError;
    use crate::traits::LlmProvider;
    use crate::types::{
        CapabilityRequest, CompletionRequest, CompletionResponse, FinishReason, ModelCapabilities,
        ModelInfo, ModelLimits, ModelPricing, RequestOptions, StreamEvent, TokenUsage,
    };
    use async_trait::async_trait;
    use futures::Stream;
    use std::collections::HashMap;
    use std::pin::Pin;

    /// Mock provider that returns a fixed-latency response.
    #[derive(Debug)]
    struct MockProvider {
        name: String,
        model_info: ModelInfo,
        latency_ms: u64,
    }

    #[async_trait]
    impl LlmProvider for MockProvider {
        async fn complete(
            &self,
            _request: CompletionRequest,
            _options: RequestOptions,
        ) -> Result<CompletionResponse, ProviderError> {
            Ok(CompletionResponse {
                content: Some("mock response".into()),
                thinking: None,
                tool_calls: vec![],
                usage: TokenUsage::default(),
                model: self.model_info.name.clone(),
                finish_reason: FinishReason::Stop,
                latency_ms: self.latency_ms,
                cache_info: None,
            })
        }

        async fn complete_stream(
            &self,
            _request: CompletionRequest,
            _options: RequestOptions,
        ) -> Result<
            Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send>>,
            ProviderError,
        > {
            unreachable!()
        }

        fn models(&self) -> &[ModelInfo] {
            std::slice::from_ref(&self.model_info)
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    fn make_test_model(name: &str, provider: &str, input_price: f64) -> ModelInfo {
        ModelInfo {
            name: name.to_owned(),
            display_name: None,
            provider: Some(provider.to_owned()),
            capabilities: ModelCapabilities::default(),
            pricing: ModelPricing {
                input_per_million: input_price,
                ..Default::default()
            },
            limits: ModelLimits::default(),
        }
    }

    /// TDD test: verify that after a successful `complete()` call, latency data
    /// persists in the router's latency_tracker so that future routing decisions
    /// can use historical latency information.
    ///
    /// Pre-fix: `complete()` cloned the tracker and recorded on the clone →
    /// the original tracker was never updated.
    /// Post-fix: `complete()` records directly on the router's tracker (wrapped
    /// in a Mutex for interior mutability).
    #[tokio::test]
    async fn router_latency_persistence() {
        let model_a = make_test_model("model-a", "provider-a", 1.0);
        let model_b = make_test_model("model-b", "provider-b", 2.0);

        let provider_a = MockProvider {
            name: "provider-a".into(),
            model_info: model_a.clone(),
            latency_ms: 450,
        };
        let provider_b = MockProvider {
            name: "provider-b".into(),
            model_info: model_b.clone(),
            latency_ms: 100,
        };

        let mut providers: HashMap<String, Box<dyn LlmProvider>> = HashMap::new();
        providers.insert("provider-a".to_owned(), Box::new(provider_a));
        providers.insert("provider-b".to_owned(), Box::new(provider_b));

        let router = ProviderRouter::new(
            providers,
            vec![model_a.clone(), model_b.clone()],
            HashMap::new(),
            "model-a".to_owned(),
            None,
        );

        // Before any requests, tracker should be empty
        assert!(
            router
                .latency_tracker
                .blocking_read()
                .avg_latency("model-a")
                .is_none(),
            "tracker should be empty before any requests"
        );

        // Route a request with Fastest preference — no history, picks first in
        // the candidate pool (model-a).
        let ctx = RouteContext {
            capabilities: Some(CapabilityRequest::default()),
            cost_preference: CostPreference::Fastest,
            ..Default::default()
        };

        let resp = router
            .complete(&ctx, CompletionRequest::default(), RequestOptions::default())
            .await
            .unwrap();

        // The request went to model-a (first in pool, no latency data)
        assert_eq!(resp.model, "model-a");

        // KEY ASSERTION: after a successful `complete()`, the latency tracker
        // MUST have recorded the model's latency for future routing decisions.
        // This is the bug: pre-fix, the tracker was never updated.
        let avg = router
            .latency_tracker
            .blocking_read()
            .avg_latency("model-a");
        assert!(
            avg.is_some(),
            "BUG: latency_tracker was not updated after complete() — \
             fastest() routing will never have data"
        );
        assert_eq!(avg.unwrap(), 450, "recorded latency should match provider's latency");

        // Route a second request — now model-b is also recorded with its
        // lower latency, and Fastest should prefer it on subsequent calls.
        // But the loop in complete() picks model-a first (primary from
        // decision), records model-a again at 450ms. So model-a still has
        // the only history. We just verify data persists across calls.
        let resp2 = router
            .complete(&ctx, CompletionRequest::default(), RequestOptions::default())
            .await
            .unwrap();
        assert_eq!(resp2.model, "model-a");

        let avg2 = router
            .latency_tracker
            .blocking_read()
            .avg_latency("model-a");
        assert!(avg2.is_some(), "latency data should persist across multiple calls");
    }

    /// Manually seed latency data and verify resolve() picks the faster model.
    #[test]
    fn router_latency_fastest_resolve() {
        let model_a = make_test_model("fast-a", "p-a", 1.0);
        let model_b = make_test_model("fast-b", "p-b", 2.0);

        let provider_a = MockProvider {
            name: "p-a".into(),
            model_info: model_a.clone(),
            latency_ms: 0, // not used — resolve() doesn't call complete()
        };
        let provider_b = MockProvider {
            name: "p-b".into(),
            model_info: model_b.clone(),
            latency_ms: 0,
        };

        let mut providers: HashMap<String, Box<dyn LlmProvider>> = HashMap::new();
        providers.insert("p-a".to_owned(), Box::new(provider_a));
        providers.insert("p-b".to_owned(), Box::new(provider_b));

        let router = ProviderRouter::new(
            providers,
            vec![model_a.clone(), model_b.clone()],
            HashMap::new(),
            "fast-a".to_owned(),
            None,
        );

        // Seed latency: model-a has high latency, model-b has low latency
        router.latency_tracker.blocking_write().record("fast-a", 999);
        router.latency_tracker.blocking_write().record("fast-b", 50);

        let ctx = RouteContext {
            capabilities: Some(CapabilityRequest::default()),
            cost_preference: CostPreference::Fastest,
            ..Default::default()
        };

        let decision = router.resolve(&ctx).unwrap();
        assert_eq!(
            decision.model, "fast-b",
            "Fastest routing should pick the historically faster model"
        );
    }
}
