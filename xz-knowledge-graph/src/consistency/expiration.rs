use sqlx::sqlite::SqlitePool;

use crate::error::KgError;
use crate::types::consistency::{ConsistencyIssue, ConsistencyIssueType, IssueSeverity};

/// Check for expired relations.
pub async fn check_expired_relations(pool: &SqlitePool) -> Result<Vec<ConsistencyIssue>, KgError> {
    #[derive(Debug, sqlx::FromRow)]
    struct ExpiredRow {
        id: String,
        source_id: String,
        target_id: String,
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    let rows: Vec<ExpiredRow> = sqlx::query_as(
        "SELECT id, source_id, target_id
         FROM relations WHERE valid_to IS NOT NULL AND valid_to < ?",
    )
    .bind(now)
    .fetch_all(pool)
    .await
    .map_err(|e| KgError::Database(e.to_string()))?;

    Ok(rows
        .into_iter()
        .map(|r| ConsistencyIssue {
            severity: IssueSeverity::Warning,
            issue_type: ConsistencyIssueType::ExpiredRelation,
            description: format!("Relation {} has expired (valid_to < now)", r.id),
            related_entities: vec![r.source_id, r.target_id],
            related_relations: vec![r.id],
        })
        .collect())
}
