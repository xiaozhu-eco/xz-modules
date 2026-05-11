use serde::{Deserialize, Serialize};

use super::confidence::Confidence;
use super::entity::{Entity, EntityType};

/// Entity search query.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EntityQuery {
    pub name_contains: Option<String>,
    pub alias_contains: Option<String>,
    pub entity_types: Option<Vec<EntityType>>,
    pub attribute_filters: Vec<AttributeFilter>,
    pub tags: Option<TagFilter>,
    pub min_confidence: Option<Confidence>,
    pub source: Option<String>,
    pub page: PageRequest,
    pub sort_by: Option<EntitySortField>,
}

/// Attribute filter condition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeFilter {
    pub key: String,
    pub operator: FilterOperator,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterOperator {
    Eq,
    Ne,
    Contains,
    StartsWith,
    EndsWith,
    Gt,
    Lt,
    Gte,
    Lte,
}

/// Tag filter with AND/OR mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagFilter {
    pub tags: Vec<String>,
    pub mode: TagFilterMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TagFilterMode {
    And,
    Or,
}

/// Pagination request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageRequest {
    pub limit: usize,
    pub offset: usize,
}

impl Default for PageRequest {
    fn default() -> Self {
        Self { limit: 50, offset: 0 }
    }
}

/// Paginated entity results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityPage {
    pub items: Vec<Entity>,
    pub total: usize,
    pub has_more: bool,
}

/// Entity sort field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntitySortField {
    Name,
    CreatedAt,
    UpdatedAt,
    EntityType,
    RelationCount,
}

/// Relation query.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RelationQuery {
    pub source_id: Option<String>,
    pub target_id: Option<String>,
    pub entity_id: Option<String>,
    pub relation_type: Option<String>,
    pub relation_types: Option<Vec<String>>,
    pub min_confidence: Option<Confidence>,
    pub valid_at: Option<u64>,
    pub page: PageRequest,
}
