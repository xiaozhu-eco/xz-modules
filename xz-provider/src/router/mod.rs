mod decision;
pub mod rule;

pub use decision::{
    CostPreference, FallbackCondition, FallbackEntry, LatencyTracker, RouteContext, RouteDecision,
};

use std::collections::HashMap;
use std::pin::Pin;

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
    latency_tracker: LatencyTracker,
    health_states: HashMap<String, HealthState>,
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
            latency_tracker: LatencyTracker::default(),
            health_states,
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
                        let mut lt = self.latency_tracker.clone();
                        lt.record(model_name, resp.latency_ms);
                        return Ok(resp);
                    }
                    Err(e) if e.is_retryable() => {
                        let mut lt = self.latency_tracker.clone();
                        lt.record_error(model_name);
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
                CostPreference::Fastest => self.latency_tracker.fastest(pool),
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
