use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::confidence::Confidence;
use super::provenance::Provenance;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum WeightStrategy {
    /// 1.0 - effective_weight (default, existing behavior)
    InverseConfidence,
    /// 1.0 / effective_weight (multiplicative)
    InverseMultiplicative,
    /// Fixed unit weight (unweighted)
    Unweighted,
    /// Custom multiplier on effective_weight
    Custom(f32),
}

impl Default for WeightStrategy {
    fn default() -> Self {
        Self::InverseConfidence
    }
}

/// Relation between two entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub relation_type: String,
    pub properties: HashMap<String, String>,
    pub confidence: Confidence,
    pub provenance: Option<Provenance>,
    pub valid_from: Option<u64>,
    pub valid_to: Option<u64>,
    pub created_at: u64,
    pub weight: Option<f32>,
}

impl Relation {
    /// Default weight = confidence.as_f32()
    pub fn effective_weight(&self) -> f32 {
        self.weight.unwrap_or_else(|| self.confidence.as_f32())
    }
}

impl WeightStrategy {
    pub fn relation_cost(self, rel: &Relation) -> f32 {
        match self {
            Self::InverseConfidence => 1.0 - rel.effective_weight().max(0.01),
            Self::InverseMultiplicative => {
                let w = rel.effective_weight().max(0.01);
                1.0 / w
            }
            Self::Unweighted => 1.0,
            Self::Custom(m) => {
                let w = rel.effective_weight() * m;
                1.0 - w.max(0.0).min(0.99)
            }
        }
    }
}
