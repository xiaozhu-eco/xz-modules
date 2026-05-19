//! FTS5 full-text search wrapper for fact recall.

use sqlx::sqlite::SqlitePool;

use crate::error::MemoryError;
use crate::types::fact::{Confidence, Fact, FactCategory, FactPage, FactRecallOptions};

/// FTS5 search helper.
pub struct FtsSearcher {
    pool: SqlitePool,
}

impl FtsSearcher {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Search facts with FTS5 and return paginated results.
    pub async fn search(
        &self,
        user_id: &str,
        query: &str,
        options: &FactRecallOptions,
    ) -> Result<FactPage, MemoryError> {
        let fts_query = format!("\"{}\"", query.replace('"', "\"\""));
        let limit = options.page.limit as i64;
        let offset = options.page.offset as i64;

        let count_sql = format!(
            "SELECT COUNT(*) FROM facts f
             JOIN facts_fts fts ON f.rowid = fts.rowid
             WHERE f.user_id = ? AND facts_fts MATCH '{}'",
            fts_query
        );
        let total: (i64,) = sqlx::query_as(&count_sql)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        let select_sql = format!(
            "SELECT f.id, f.user_id, f.category, f.subject, f.predicate, f.object, f.confidence,
                    f.source_session, f.created_at, f.updated_at, f.version
             FROM facts f
             JOIN facts_fts fts ON f.rowid = fts.rowid
             WHERE f.user_id = ? AND facts_fts MATCH '{}'
             ORDER BY f.updated_at DESC
             LIMIT ? OFFSET ?",
            fts_query
        );

        let rows: Vec<FtsFactRow> = sqlx::query_as(&select_sql)
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| MemoryError::database_with_source(e.to_string(), e))?;

        let total = total.0 as usize;
        let items: Vec<Fact> = rows.into_iter().map(|r| r.into()).collect();
        let has_more = (offset as usize + limit as usize) < total;

        Ok(FactPage { items, total, has_more })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct FtsFactRow {
    id: String,
    user_id: String,
    category: String,
    subject: String,
    predicate: String,
    object: String,
    confidence: f32,
    source_session: Option<String>,
    created_at: i64,
    updated_at: i64,
    version: i64,
}

impl From<FtsFactRow> for Fact {
    fn from(r: FtsFactRow) -> Self {
        Self {
            id: r.id,
            user_id: r.user_id,
            category: str_to_fact_category(&r.category),
            subject: r.subject,
            predicate: r.predicate,
            object: r.object,
            confidence: Confidence::from_f32(r.confidence),
            source_session: r.source_session,
            created_at: r.created_at as u64,
            updated_at: r.updated_at as u64,
            version: r.version as u64,
        }
    }
}

fn str_to_fact_category(s: &str) -> FactCategory {
    match s {
        "Preference" => FactCategory::Preference,
        "PersonalInfo" => FactCategory::PersonalInfo,
        "Relationship" => FactCategory::Relationship,
        "Event" => FactCategory::Event,
        "Schedule" => FactCategory::Schedule,
        "Health" => FactCategory::Health,
        "Location" => FactCategory::Location,
        other => FactCategory::Custom(other.to_string()),
    }
}
