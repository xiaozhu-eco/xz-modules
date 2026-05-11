use serde::{Deserialize, Serialize};

/// Data provenance information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    pub source_text: String,
    pub source_id: String,
    pub extraction_method: ExtractionMethod,
    pub extracted_at: u64,
    pub extractor_version: Option<String>,
}

/// How the data was extracted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtractionMethod {
    Lm {
        model: String,
        prompt_version: String,
    },
    Manual,
    Reconciliation,
    RuleBased {
        rule_name: String,
    },
}
