use async_trait::async_trait;

use crate::error::RerankError;
use crate::traits::{RecencyFunction, SignalPlugin};
use crate::types::RerankCandidate;

use crate::types::RecencyMode;

/// 时间近因性信号
#[derive(Debug)]
pub struct RecencySignal {
    mode: RecencyMode,
    now_ms: u64,
}

impl RecencySignal {
    pub fn new(mode: RecencyMode) -> Self {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self { mode, now_ms }
    }

    pub fn with_now(mut self, now_ms: u64) -> Self {
        self.now_ms = now_ms;
        self
    }
}

#[async_trait]
impl SignalPlugin for RecencySignal {
    fn name(&self) -> &str {
        "recency"
    }

    async fn score(
        &self,
        _query: &str,
        candidate: &RerankCandidate,
    ) -> Result<f32, RerankError> {
        let created_at = match candidate.created_at {
            Some(t) => t,
            None => return Ok(0.5), // 无时间信息，中性分数
        };

        let age_seconds = (self.now_ms.saturating_sub(created_at) as f64) / 1000.0;

        let score = match &self.mode {
            RecencyMode::NoDecay => 1.0,
            RecencyMode::LinearDecay { max_age_days } => {
                let max_age_s = max_age_days * 86400.0;
                (1.0 - age_seconds / max_age_s).max(0.0)
            }
            RecencyMode::ExponentialDecay { decay_rate } => {
                (-decay_rate * age_seconds).exp()
            }
        };

        Ok(score as f32)
    }
}
