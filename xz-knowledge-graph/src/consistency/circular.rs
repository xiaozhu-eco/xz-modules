use sqlx::sqlite::SqlitePool;

use crate::error::KgError;
use crate::types::consistency::{ConsistencyIssue, ConsistencyIssueType, IssueSeverity};

/// Check for self-referencing relations.
pub async fn check_self_referencing(pool: &SqlitePool) -> Result<Vec<ConsistencyIssue>, KgError> {
    #[derive(Debug, sqlx::FromRow)]
    struct SelfRefRow {
        id: String,
        source_id: String,
    }

    let rows: Vec<SelfRefRow> = sqlx::query_as(
        "SELECT id, source_id FROM relations WHERE source_id = target_id",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| KgError::Database(e.to_string()))?;

    Ok(rows
        .into_iter()
        .map(|r| ConsistencyIssue {
            severity: IssueSeverity::Warning,
            issue_type: ConsistencyIssueType::SelfReferencing,
            description: format!("Relation {} self-references entity {}", r.id, r.source_id),
            related_entities: vec![r.source_id],
            related_relations: vec![r.id],
        })
        .collect())
}
