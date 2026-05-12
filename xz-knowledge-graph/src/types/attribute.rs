use serde::{Deserialize, Serialize};

use super::confidence::Confidence;
use super::provenance::Provenance;

/// Attribute value with confidence and provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeValue {
    pub value: String,
    pub confidence: Confidence,
    pub provenance: Option<Provenance>,
}

impl AttributeValue {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            confidence: Confidence::Medium,
            provenance: None,
        }
    }

    pub fn with_confidence(mut self, confidence: Confidence) -> Self {
        self.confidence = confidence;
        self
    }
}
