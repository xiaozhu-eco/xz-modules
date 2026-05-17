use async_trait::async_trait;
use std::collections::HashMap;

use crate::error::RerankError;
use crate::traits::SignalPlugin;
use crate::types::RerankCandidate;

/// 元数据匹配信号
#[derive(Debug)]
pub struct MetadataMatchSignal {
    /// 需要匹配的元数据字段及权重
    field_weights: HashMap<String, f32>,
}

impl MetadataMatchSignal {
    pub fn new(field_weights: HashMap<String, f32>) -> Self {
        Self { field_weights }
    }
}

impl Default for MetadataMatchSignal {
    fn default() -> Self {
        Self {
            field_weights: HashMap::new(),
        }
    }
}

#[async_trait]
impl SignalPlugin for MetadataMatchSignal {
    fn name(&self) -> &str {
        "metadata_match"
    }

    fn weight_key(&self) -> &'static str {
        "metadata_match"
    }

    async fn score(
        &self,
        _query: &str,
        candidate: &RerankCandidate,
    ) -> Result<f32, RerankError> {
        if self.field_weights.is_empty() {
            return Ok(1.0);
        }

        let mut total_weight = 0.0f32;
        let mut matched_weight = 0.0f32;

        for (field, weight) in &self.field_weights {
            total_weight += weight;
            if candidate.metadata.contains_key(field.as_str()) {
                matched_weight += weight;
            }
        }

        if total_weight <= f32::EPSILON {
            return Ok(1.0);
        }
        Ok(matched_weight / total_weight)
    }
}
