use sqlx::sqlite::SqlitePool;

use crate::error::KgError;
use crate::types::consistency::{ConsistencyIssue, ConsistencyIssueType, IssueSeverity};

/// Check for orphan relations (relations referencing non-existent entities).
pub async fn check_orphan_relations(pool: &SqlitePool) -> Result<Vec<ConsistencyIssue>, KgError> {
    #[derive(Debug, sqlx::FromRow)]
    struct OrphanRow {
        id: String,
        source_id: String,
        target_id: String,
    }

    let rows: Vec<OrphanRow> = sqlx::query_as(
        "SELECT r.id, r.source_id, r.target_id
         FROM relations r
         LEFT JOIN entities e1 ON r.source_id = e1.id
         LEFT JOIN entities e2 ON r.target_id = e2.id
         WHERE e1.id IS NULL OR e2.id IS NULL",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| KgError::Database(e.to_string()))?;

    Ok(rows
        .into_iter()
        .map(|r| ConsistencyIssue {
            severity: IssueSeverity::Error,
            issue_type: ConsistencyIssueType::OrphanRelation,
            description: format!("Relation {} references a non-existent entity", r.id),
            related_entities: vec![r.source_id, r.target_id],
            related_relations: vec![r.id],
        })
        .collect())
}

/// Check for orphan entities (entities with no relations).
pub async fn check_orphan_entities(pool: &SqlitePool) -> Result<Vec<ConsistencyIssue>, KgError> {
    #[derive(Debug, sqlx::FromRow)]
    struct OrphanEntityRow {
        id: String,
        name: String,
    }

    let rows: Vec<OrphanEntityRow> = sqlx::query_as(
        "SELECT e.id, e.name FROM entities e
         WHERE e.id NOT IN (SELECT source_id FROM relations)
           AND e.id NOT IN (SELECT target_id FROM relations)",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| KgError::Database(e.to_string()))?;

    Ok(rows
        .into_iter()
        .map(|r| ConsistencyIssue {
            severity: IssueSeverity::Info,
            issue_type: ConsistencyIssueType::OrphanEntity,
            description: format!("Entity {} ({}) has no relations", r.name, r.id),
            related_entities: vec![r.id],
            related_relations: vec![],
        })
        .collect())
}
