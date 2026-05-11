use sqlx::sqlite::SqlitePool;

use crate::error::KgError;
use crate::types::consistency::{ConsistencyIssue, ConsistencyIssueType, IssueSeverity};

/// Check for duplicate entities (same name with different IDs).
pub async fn check_duplicate_entities(pool: &SqlitePool) -> Result<Vec<ConsistencyIssue>, KgError> {
    #[derive(Debug, sqlx::FromRow)]
    struct DuplicateRow {
        name: String,
        ids: String, // JSON array of IDs
    }

    let rows: Vec<DuplicateRow> = sqlx::query_as(
        "SELECT name, json_group_array(id) as ids
         FROM entities
         GROUP BY name
         HAVING COUNT(*) > 1",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| KgError::Database(e.to_string()))?;

    Ok(rows
        .into_iter()
        .map(|r| {
            let ids: Vec<String> =
                serde_json::from_str(&r.ids).unwrap_or_else(|_| vec![r.ids.clone()]);
            ConsistencyIssue {
                severity: IssueSeverity::Warning,
                issue_type: ConsistencyIssueType::DuplicateEntity,
                description: format!(
                    "Multiple entities with name '{}': {}",
                    r.name,
                    ids.join(", ")
                ),
                related_entities: ids,
                related_relations: vec![],
            }
        })
        .collect())
}
