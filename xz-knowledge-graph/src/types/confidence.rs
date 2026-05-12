use serde::{Deserialize, Serialize};

/// Confidence level for entities, relations, and attributes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confidence {
    Speculative,
    Low,
    Medium,
    High,
    Confirmed,
}

impl Confidence {
    pub fn as_f32(self) -> f32 {
        match self {
            Self::Speculative => 0.15,
            Self::Low => 0.35,
            Self::Medium => 0.60,
            Self::High => 0.85,
            Self::Confirmed => 1.0,
        }
    }

    pub fn from_f32(v: f32) -> Self {
        if v >= 1.0 {
            Self::Confirmed
        } else if v >= 0.85 {
            Self::High
        } else if v >= 0.6 {
            Self::Medium
        } else if v >= 0.35 {
            Self::Low
        } else {
            Self::Speculative
        }
    }
}
