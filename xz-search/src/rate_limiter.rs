use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use tokio::sync::Mutex;

use crate::error::SearchError;

/// 搜索引擎速率限制器 — 令牌桶算法
#[derive(Debug)]
pub struct SearchRateLimiter {
    /// 每秒请求数（QPS）
    qps_limit: f64,
    /// 每日请求数
    daily_limit: u64,
    /// 当前可用令牌（浮点，支持部分令牌）
    tokens: Mutex<f64>,
    /// 上次补充令牌的时间
    last_refill: Mutex<Instant>,
    /// 今日已用请求数
    daily_count: AtomicU64,
    /// 今日最后一次重置
    last_daily_reset: Mutex<Instant>,
}

impl SearchRateLimiter {
    pub fn new(qps_limit: f64, daily_limit: u64) -> Self {
        Self {
            qps_limit,
            daily_limit,
            tokens: Mutex::new(qps_limit),
            last_refill: Mutex::new(Instant::now()),
            daily_count: AtomicU64::new(0),
            last_daily_reset: Mutex::new(Instant::now()),
        }
    }

    /// 获取许可（如果超过限制则返回错误）
    pub async fn acquire(&self, engine_name: &str) -> Result<(), SearchError> {
        // 检查日限额
        self.check_daily_reset().await;
        let daily = self.daily_count.load(Ordering::Relaxed);
        if daily >= self.daily_limit {
            return Err(SearchError::RateLimit {
                engine: engine_name.to_string(),
                retry_after_ms: self.until_next_day_ms().await,
            });
        }

        // 令牌桶
        let now = Instant::now();
        {
            let mut tokens = self.tokens.lock().await;
            let mut last = self.last_refill.lock().await;

            let elapsed = now.duration_since(*last).as_secs_f64();
            *tokens = (*tokens + elapsed * self.qps_limit).min(self.qps_limit);
            *last = now;

            if *tokens < 1.0 {
                let wait_ms = ((1.0 - *tokens) / self.qps_limit * 1000.0).ceil() as u64;
                return Err(SearchError::RateLimit {
                    engine: engine_name.to_string(),
                    retry_after_ms: wait_ms,
                });
            }

            *tokens -= 1.0;
        }

        self.daily_count.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    async fn check_daily_reset(&self) {
        let mut last = self.last_daily_reset.lock().await;
        let now = Instant::now();
        if now.duration_since(*last) > Duration::from_secs(86400) {
            self.daily_count.store(0, Ordering::Relaxed);
            *last = now;
        }
    }

    async fn until_next_day_ms(&self) -> u64 {
        let last = self.last_daily_reset.lock().await;
        let elapsed = last.elapsed().as_secs();
        86400u64.saturating_sub(elapsed) * 1000
    }
}

/// 为 SearchRouter 增强的限流搜索辅助函数
pub struct LimitedSearch<'a> {
    pub engine_name: &'a str,
    pub limiter: Option<&'a SearchRateLimiter>,
}

impl<'a> LimitedSearch<'a> {
    pub async fn acquire(&self) -> Result<(), SearchError> {
        if let Some(lim) = self.limiter {
            lim.acquire(self.engine_name).await?;
        }
        Ok(())
    }
}
