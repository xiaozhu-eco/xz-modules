use serde::{Deserialize, Serialize};

use crate::traits::RecencyFunction;

/// 重排序配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankConfig {
    /// 最终返回的最大候选项数
    pub top_k: usize,
    /// 最低分数阈值
    pub min_score: Option<f32>,
    /// 是否返回分数分解
    pub include_score_breakdown: bool,
    /// 是否按通道区分近因性衰减
    pub recency_mode: Option<RecencyMode>,
}

impl Default for RerankConfig {
    fn default() -> Self {
        Self {
            top_k: 10,
            min_score: Some(0.2),
            include_score_breakdown: false,
            recency_mode: Some(RecencyMode::ExponentialDecay { decay_rate: 0.01 }),
        }
    }
}

/// 近因性衰减模式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecencyMode {
    /// 不衰减
    NoDecay,
    /// 线性衰减
    LinearDecay { max_age_days: f64 },
    /// 指数衰减
    ExponentialDecay { decay_rate: f64 },
}

/// 按通道区分的近因性规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelRecencyRule {
    pub channel: String,
    pub mode: RecencyMode,
}
