use serde::{Deserialize, Serialize};

/// Consistency issue found during validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyIssue {
    pub severity: IssueSeverity,
    pub issue_type: ConsistencyIssueType,
    pub description: String,
    pub related_entities: Vec<String>,
    pub related_relations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsistencyIssueType {
    OrphanRelation,
    SelfReferencing,
    CircularReference,
    DuplicateEntity,
    ConflictingAttribute,
    ExpiredRelation,
    OrphanEntity,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IssueSeverity {
    Info,
    Warning,
    Error,
}
