use std::{collections::HashMap, time::Instant};

use crate::error::NotifError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitAction {
    Delay,
    Drop,
}

#[derive(Debug, Clone)]
pub struct ChannelRateLimit {
    pub max_per_second: u32,
    pub burst: u32,
    pub action: RateLimitAction,
}

#[derive(Debug, Clone, Default)]
pub struct RateLimitConfig {
    pub channels: HashMap<String, ChannelRateLimit>,
}

#[derive(Debug, Clone)]
pub struct TokenBucket {
    pub capacity: u32,
    pub refill_rate: f64,
    pub tokens: f64,
    pub last_refill: Instant,
}

#[derive(Debug, Clone)]
pub struct RateLimiter {
    pub buckets: HashMap<String, TokenBucket>,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        let now = Instant::now();
        let buckets = config
            .channels
            .into_iter()
            .map(|(channel, limit)| {
                let capacity = limit.burst.max(1);
                (
                    channel,
                    TokenBucket {
                        capacity,
                        refill_rate: limit.max_per_second as f64,
                        tokens: capacity as f64,
                        last_refill: now,
                    },
                )
            })
            .collect();

        Self { buckets }
    }

    pub fn check(&mut self, channel_id: &str) -> Result<(), NotifError> {
        let bucket = self
            .buckets
            .get_mut(channel_id)
            .ok_or_else(|| NotifError::ChannelNotFound(channel_id.to_string()))?;

        Self::refill(bucket);

        if bucket.tokens < 1.0 {
            let retry_after_ms = if bucket.refill_rate <= 0.0 {
                u64::MAX
            } else {
                ((1.0 - bucket.tokens) / bucket.refill_rate * 1000.0) as u64
            };
            return Err(NotifError::RateLimited { channel: channel_id.to_string(), retry_after_ms });
        }

        bucket.tokens -= 1.0;
        Ok(())
    }

    fn refill(bucket: &mut TokenBucket) {
        let elapsed_secs = bucket.last_refill.elapsed().as_secs_f64();
        if elapsed_secs > 0.0 {
            bucket.tokens = (bucket.tokens + elapsed_secs * bucket.refill_rate).min(bucket.capacity as f64);
            bucket.last_refill = Instant::now();
        }
    }
}
