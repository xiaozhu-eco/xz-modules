use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt::Debug;
use tokio::sync::RwLock;

/// 搜索结果反馈 — 收集用户点击/弃用信号
#[async_trait]
pub trait SearchFeedback: Send + Sync + Debug {
    /// 记录用户点击了某条结果
    async fn record_click(&self, query: &str, item_url: &str);

    /// 记录用户标记某条结果不相关
    async fn record_irrelevant(&self, query: &str, item_url: &str);

    /// 获取历史点击权重（用于排序调整）
    async fn get_url_weight(&self, url: &str) -> f32;
}

/// 内存实现的搜索反馈收集器
#[derive(Debug)]
pub struct MemorySearchFeedback {
    /// url → 累计点击次数
    click_counts: RwLock<HashMap<String, u64>>,
    /// url → 累计不相关标记次数
    irrelevant_counts: RwLock<HashMap<String, u64>>,
    /// 总反馈数
    total_feedback: RwLock<u64>,
}

impl MemorySearchFeedback {
    pub fn new() -> Self {
        Self {
            click_counts: RwLock::new(HashMap::new()),
            irrelevant_counts: RwLock::new(HashMap::new()),
            total_feedback: RwLock::new(0),
        }
    }

    pub fn click_stats(&self) -> HashMap<String, u64> {
        self.click_counts
            .try_read()
            .map(|counts| counts.clone())
            .unwrap_or_default()
    }
}

impl Default for MemorySearchFeedback {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchFeedback for MemorySearchFeedback {
    async fn record_click(&self, _query: &str, item_url: &str) {
        *self.click_counts.write().await
            .entry(item_url.to_string())
            .or_default() += 1;
        *self.total_feedback.write().await += 1;
    }

    async fn record_irrelevant(&self, _query: &str, item_url: &str) {
        *self.irrelevant_counts.write().await
            .entry(item_url.to_string())
            .or_default() += 1;
        *self.total_feedback.write().await += 1;
    }

    async fn get_url_weight(&self, url: &str) -> f32 {
        let clicks = self.click_counts.read().await;
        let irrelevants = self.irrelevant_counts.read().await;

        let c = clicks.get(url).copied().unwrap_or(0) as f32;
        let i = irrelevants.get(url).copied().unwrap_or(0) as f32;

        if c + i == 0.0 {
            return 1.0; // 无反馈，中性权重
        }

        // 简单计算：weight = 1 + log2(1 + clicks) - log2(1 + irrelevants)
        let score = 1.0 + (1.0 + c).log2() - (1.0 + i).log2();
        score.max(0.1).min(5.0)
    }
}
