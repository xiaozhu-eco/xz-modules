use serde::{Deserialize, Serialize};

use super::query::PageRequest;

/// A structured fact (subject-predicate-object triple).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    pub id: String,
    pub user_id: String,
    pub category: FactCategory,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: Confidence,
    pub source_session: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
    pub version: u64,
}

/// Category of a fact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FactCategory {
    Preference,
    PersonalInfo,
    Relationship,
    Event,
    Schedule,
    Health,
    Location,
    /// Novel character state (used by the domain character module).
    Character,
    Custom(String),
}

/// Confidence level of a fact.
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
            Self::Medium => 0.6,
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

/// Options for fact recall.
#[derive(Debug, Clone)]
pub struct FactRecallOptions {
    pub page: PageRequest,
    pub min_confidence: Option<Confidence>,
    pub categories: Option<Vec<FactCategory>>,
    pub sort_by: FactSortField,
}

impl Default for FactRecallOptions {
    fn default() -> Self {
        Self {
            page: PageRequest::default(),
            min_confidence: None,
            categories: None,
            sort_by: FactSortField::UpdatedAt,
        }
    }
}

/// Sort field for fact queries.
#[derive(Debug, Clone, Copy)]
pub enum FactSortField {
    UpdatedAt,
    Confidence,
    CreatedAt,
}

/// Compaction strategy.
#[derive(Debug, Clone)]
pub enum CompactionStrategy {
    MergeSimilar,
    RemoveLowConfidence(f32),
    RemoveOld(u64),
}

/// Result of a compaction operation.
#[derive(Debug, Clone, Default)]
pub struct CompactionResult {
    pub facts_merged: usize,
    pub facts_removed: usize,
    pub facts_kept: usize,
}

/// Paginated fact results.
#[derive(Debug, Clone)]
pub struct FactPage {
    pub items: Vec<Fact>,
    pub total: usize,
    pub has_more: bool,
}
