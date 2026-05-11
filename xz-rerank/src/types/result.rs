use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::RerankCandidate;

/// 重排序结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankResult {
    /// 排序后的命中列表（按 score 降序）
    pub hits: Vec<RerankHit>,
    /// 统计信息
    pub stats: RerankStats,
    /// 使用的重排序器
    pub reranker: String,
    /// 总延迟（毫秒）
    pub latency_ms: u64,
}

/// 单个重排序命中
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankHit {
    /// 对应 RerankCandidate.id
    pub candidate_id: String,
    /// 综合分数 [0, 1]
    pub score: f32,
    /// 分数分解
    pub score_breakdown: Option<ScoreBreakdown>,
    /// 原始候选项（透传）
    pub candidate: RerankCandidate,
}

/// 分数分解
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub signals: Vec<SignalScore>,
    pub final_score: f32,
}

/// 单个信号分数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalScore {
    pub name: String,
    pub raw_score: f32,
    pub weight: f32,
    pub contribution: f32, // raw_score * weight
}

/// 重排序统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankStats {
    /// 输入候选项数
    pub total_candidates: usize,
    /// 被 min_score 过滤掉的
    pub filtered_out: usize,
    /// 最高分
    pub max_score: f32,
    /// 最低分
    pub min_score: f32,
    /// 平均分
    pub avg_score: f32,
    /// 中位数
    pub median_score: f32,
    /// 各信号耗时（微秒）
    pub signal_timings: HashMap<String, u64>,
}
