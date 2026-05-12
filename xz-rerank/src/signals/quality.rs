use async_trait::async_trait;

use crate::error::RerankError;
use crate::traits::SignalPlugin;
use crate::types::RerankCandidate;

/// 内容质量启发式信号
#[derive(Debug)]
pub struct ContentQualitySignal;

#[async_trait]
impl SignalPlugin for ContentQualitySignal {
    fn name(&self) -> &str {
        "content_quality"
    }

    async fn score(
        &self,
        _query: &str,
        candidate: &RerankCandidate,
    ) -> Result<f32, RerankError> {
        let content = &candidate.content;
        let len = content.len() as f32;

        let optimal_min = 200.0f32;
        let optimal_max = 2000.0f32;

        if len < optimal_min {
            Ok(len / optimal_min)
        } else if len <= optimal_max {
            Ok(1.0)
        } else {
            Ok((optimal_max / len).max(0.8))
        }
    }
}
