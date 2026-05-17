use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Instant;

use crate::error::RerankError;
use crate::signals::{
    ContentQualitySignal, KeywordOverlapSignal, MetadataMatchSignal, RecencySignal,
    VectorSimilaritySignal,
};
use crate::traits::{Reranker, RerankerBackendType, RerankerInfo, RerankerPricing, SignalPlugin};
use crate::types::{
    ChannelRecencyRule, RecencyMode, RerankCandidate, RerankConfig,
    RerankHit, RerankResult, RerankStats, ScoreBreakdown, SignalScore, SignalWeights,
};

/// 本地多信号融合重排序器
pub struct LocalSignalReranker {
    weights: SignalWeights,
    signals: Vec<Box<dyn SignalPlugin>>,
    default_recency_mode: RecencyMode,
    channel_recency: Vec<ChannelRecencyRule>,
    info: RerankerInfo,
}

impl std::fmt::Debug for LocalSignalReranker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalSignalReranker")
            .field("weights", &self.weights)
            .field("signal_names", &self.signals.iter().map(|s| s.name().to_string()).collect::<Vec<_>>())
            .field("default_recency_mode", &self.default_recency_mode)
            .finish()
    }
}

impl LocalSignalReranker {
    /// 创建新的本地重排序器
    pub fn new(weights: SignalWeights) -> Self {
        Self {
            weights,
            signals: Vec::new(),
            default_recency_mode: RecencyMode::ExponentialDecay { decay_rate: 0.01 },
            channel_recency: Vec::new(),
            info: RerankerInfo {
                name: "local-signal".into(),
                display_name: "Local Signal Reranker".into(),
                backend_type: RerankerBackendType::Local,
                supports_batch: true,
                max_candidates: None,
                pricing: Some(RerankerPricing {
                    cost_per_search: 0.0,
                }),
            },
        }
    }

    /// 添加自定义信号
    pub fn with_signal(mut self, signal: Box<dyn SignalPlugin>) -> Self {
        self.signals.push(signal);
        self
    }

    /// 按通道配置近因性衰减
    pub fn with_channel_recency(mut self, rules: Vec<ChannelRecencyRule>) -> Self {
        self.channel_recency = rules;
        self
    }

    /// 设置默认近因性模式
    pub fn with_recency_mode(mut self, mode: RecencyMode) -> Self {
        self.default_recency_mode = mode;
        self
    }

    #[allow(dead_code)]
    fn get_recency_mode_for(&self, channel: Option<&String>) -> RecencyMode {
        if let Some(ch) = channel {
            for rule in &self.channel_recency {
                if rule.channel == *ch {
                    return rule.mode.clone();
                }
            }
        }
        self.default_recency_mode.clone()
    }

    async fn compute_scores(
        &self,
        query: &str,
        candidates: &[RerankCandidate],
    ) -> Result<Vec<Vec<f32>>, RerankError> {
        use futures::future::join_all;

        let futures: Vec<_> = self
            .signals
            .iter()
            .map(|signal| {
                let q = query.to_string();
                let c = candidates.to_vec();
                async move { signal.score_batch(&q, &c).await }
            })
            .collect();

        let results = join_all(futures).await;
        let mut signal_scores = Vec::with_capacity(results.len());
        for result in results {
            signal_scores.push(result?);
        }

        Ok(signal_scores)
    }

    fn weighted_sum(
        &self,
        signal_scores: &[Vec<f32>],
        weights: &SignalWeights,
    ) -> Vec<f32> {
        let n = signal_scores.first().map(|s| s.len()).unwrap_or(0);
        let mut final_scores = vec![0.0f32; n];

        for (signal_idx, signal_score_vec) in signal_scores.iter().enumerate() {
            let weight_key = self.signals.get(signal_idx).map(|s| s.weight_key()).unwrap_or("");
            let w = weights.get_weight_by_name(weight_key);
            for (i, &s) in signal_score_vec.iter().enumerate() {
                final_scores[i] += s * w;
            }
        }

        final_scores
    }
}

impl Default for LocalSignalReranker {
    fn default() -> Self {
        let weights = SignalWeights::default();
        Self {
            weights: weights.clone(),
            signals: vec![
                Box::new(KeywordOverlapSignal),
                Box::new(VectorSimilaritySignal::new()),
                Box::new(MetadataMatchSignal::default()),
                Box::new(ContentQualitySignal),
                Box::new(RecencySignal::new(
                    RecencyMode::ExponentialDecay { decay_rate: 0.01 },
                )),
            ],
            default_recency_mode: RecencyMode::ExponentialDecay { decay_rate: 0.01 },
            channel_recency: Vec::new(),
            info: RerankerInfo {
                name: "local-signal".into(),
                display_name: "Local Signal Reranker".into(),
                backend_type: RerankerBackendType::Local,
                supports_batch: true,
                max_candidates: None,
                pricing: Some(RerankerPricing {
                    cost_per_search: 0.0,
                }),
            },
        }
    }
}

#[async_trait]
impl Reranker for LocalSignalReranker {
    async fn rerank(
        &self,
        query: &str,
        candidates: Vec<RerankCandidate>,
        config: &RerankConfig,
    ) -> Result<RerankResult, RerankError> {
        let start = Instant::now();

        if candidates.is_empty() {
            return Err(RerankError::EmptyCandidates);
        }

        let total_candidates = candidates.len();

        // 1. 为每个候选项按通道选择 recency mode
        // (recency signal 已在构建时设置，在 default 使用)

        // 2. 并行计算所有信号
        let signal_scores = self.compute_scores(query, &candidates).await?;

        // 3. 加权求和（按信号名称查找权重）
        let final_scores = self.weighted_sum(&signal_scores, &self.weights);

        // 4. 组合并排序
        let mut scored: Vec<(usize, f32)> = final_scores
            .iter()
            .enumerate()
            .map(|(i, &s)| (i, s))
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // 5. 应用 min_score 过滤和 top_k 截断
        let min_score = config.min_score.unwrap_or(0.0);
        let mut filtered_out = 0usize;
        let mut hits = Vec::new();

        for (idx, score) in scored {
            if score < min_score {
                filtered_out += 1;
                continue;
            }
            if hits.len() >= config.top_k {
                continue;
            }

            let candidate = &candidates[idx];

            let score_breakdown = if config.include_score_breakdown {
                let signals: Vec<SignalScore> = signal_scores
                    .iter()
                    .enumerate()
                    .map(|(si, scores)| {
                        let raw_score = scores[idx];
                        let weight = self.signals.get(si)
                    .map(|s| self.weights.get_weight_by_name(s.weight_key()))
                    .unwrap_or(0.0);
                        SignalScore {
                            name: self.get_signal_name(si),
                            raw_score,
                            weight,
                            contribution: raw_score * weight,
                        }
                    })
                    .collect();

                Some(ScoreBreakdown {
                    signals,
                    final_score: score,
                })
            } else {
                None
            };

            hits.push(RerankHit {
                candidate_id: candidate.id.clone(),
                score,
                score_breakdown,
                candidate: candidate.clone(),
            });
        }

        // 计算统计信息
        let final_scores_vec: Vec<f32> = hits.iter().map(|h| h.score).collect();
        let n = final_scores_vec.len();
        let max_score = final_scores_vec.first().copied().unwrap_or(0.0);
        let min_score_final = final_scores_vec.last().copied().unwrap_or(0.0);
        let avg_score = if n > 0 {
            final_scores_vec.iter().sum::<f32>() / n as f32
        } else {
            0.0
        };
        let mut sorted = final_scores_vec.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median_score = if n > 0 { sorted[n / 2] } else { 0.0 };

        Ok(RerankResult {
            hits,
            stats: RerankStats {
                total_candidates,
                filtered_out,
                max_score,
                min_score: min_score_final,
                avg_score,
                median_score,
                signal_timings: HashMap::new(),
            },
            reranker: self.info.name.clone(),
            latency_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn reranker_info(&self) -> &RerankerInfo {
        &self.info
    }
}

impl LocalSignalReranker {
    fn get_signal_name(&self, idx: usize) -> String {
        self.signals
            .get(idx)
            .map(|s| s.name().to_string())
            .unwrap_or_else(|| format!("signal_{idx}"))
    }
}
