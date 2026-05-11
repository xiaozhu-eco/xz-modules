use serde::{Deserialize, Serialize};

/// Result of a batch import operation.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImportResult {
    pub entities_created: usize,
    pub entities_updated: usize,
    pub entities_skipped: usize,
    pub relations_created: usize,
    pub relations_updated: usize,
    pub conflicts: Vec<Conflict>,
}

/// Upsert conflict information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub entity_id: String,
    pub field: String,
    pub existing_value: String,
    pub new_value: String,
}

/// Upsert result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpsertResult {
    Created,
    Updated {
        changed_fields: Vec<String>,
        conflicts: Vec<Conflict>,
    },
    Unchanged,
}

/// Attribute merge strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MergeStrategy {
    Replace,
    Keep,
    #[allow(dead_code)]
    HigherConfidence,
    #[allow(dead_code)]
    Append,
    #[allow(dead_code)]
    Conflict,
}
