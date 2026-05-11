use crate::types::{CapabilityRequest, ModelInfo};

/// 路由决策上下文 —— 一次路由需要的所有输入
#[derive(Debug, Clone, Default)]
pub struct RouteContext {
    pub capabilities: Option<CapabilityRequest>,
    pub named_route: Option<String>,
    pub model: Option<String>,
    pub cost_preference: CostPreference,
}

/// 成本偏好
#[derive(Debug, Clone, Default)]
pub enum CostPreference {
    Cheapest,
    Fastest,
    Balanced,
    #[default]
    NoPreference,
}

/// 路由决策结果
#[derive(Debug, Clone)]
pub struct RouteDecision {
    pub model: String,
    pub provider: String,
    pub fallback_chain: Vec<FallbackEntry>,
}

/// 回退条目
#[derive(Debug, Clone)]
pub struct FallbackEntry {
    pub model: String,
    pub provider: String,
    pub condition: FallbackCondition,
}

/// 回退触发条件
#[derive(Debug, Clone)]
pub enum FallbackCondition {
    Always,
    RateLimitOnly,
    ErrorStatus(Vec<u16>),
}

impl RouteDecision {
    pub fn iter_entries(&self) -> impl Iterator<Item = (&str, &str)> {
        let primary = std::iter::once((self.provider.as_str(), self.model.as_str()));
        let fallbacks = self
            .fallback_chain
            .iter()
            .map(|e| (e.provider.as_str(), e.model.as_str()));
        primary.chain(fallbacks)
    }
}

/// 延迟追踪器 —— 记录每个 model 的历史延迟，供 Fastest 路由决策使用
#[derive(Debug, Clone)]
pub struct LatencyTracker {
    history: std::collections::HashMap<String, std::collections::VecDeque<u64>>,
    max_history: usize,
}

impl LatencyTracker {
    pub fn new(max_history: usize) -> Self {
        Self {
            history: std::collections::HashMap::new(),
            max_history,
        }
    }

    pub fn record(&mut self, model: &str, latency_ms: u64) {
        let entry = self.history.entry(model.to_owned()).or_default();
        entry.push_back(latency_ms);
        if entry.len() > self.max_history {
            entry.pop_front();
        }
    }

    pub fn record_error(&mut self, model: &str) {
        let entry = self.history.entry(model.to_owned()).or_default();
        entry.push_back(u64::MAX / 2);
        if entry.len() > self.max_history {
            entry.pop_front();
        }
    }

    pub fn avg_latency(&self, model: &str) -> Option<u64> {
        let history = self.history.get(model)?;
        if history.is_empty() {
            return None;
        }
        let sum: u64 = history.iter().sum();
        Some(sum / history.len() as u64)
    }

    pub fn fastest(&self, models: &[&ModelInfo]) -> Option<String> {
        models
            .iter()
            .filter(|m| self.history.contains_key(&m.name))
            .min_by_key(|m| self.avg_latency(&m.name).unwrap_or(u64::MAX))
            .or_else(|| models.first())
            .map(|m| m.name.clone())
    }
}

impl Default for LatencyTracker {
    fn default() -> Self {
        Self::new(10)
    }
}
