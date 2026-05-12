use std::collections::HashMap;

use sqlx::sqlite::SqlitePool;

use crate::error::KgError;
use crate::types::attribute::AttributeValue;
use crate::types::entity::{Entity, EntityType};

/// FTS5 full-text search wrapper over the entities_fts virtual table.
#[derive(Debug)]
pub struct FtsSearcher {
    pool: SqlitePool,
    min_query_length: usize,
}

impl FtsSearcher {
    pub fn new(pool: SqlitePool, min_query_length: usize) -> Self {
        Self {
            pool,
            min_query_length,
        }
    }

    /// Search entities by keyword using FTS5.
    pub async fn search(&self, keyword: &str, limit: usize) -> Result<Vec<(Entity, f64)>, KgError> {
        if keyword.trim().len() < self.min_query_length {
            return Ok(vec![]);
        }

        let query = format!("{}*", keyword);
        let rows: Vec<FtsSearchRow> = sqlx::query_as(
            "SELECT e.id, e.name, e.entity_type, e.attributes_json, e.description,
                    e.created_at, e.updated_at, e.version, e.source, e.tags_json, e.aliases_json,
                    f.rank
             FROM entities_fts f
             JOIN entities e ON f.rowid = e.rowid
             WHERE entities_fts MATCH ?
             ORDER BY rank
             LIMIT ?",
        )
        .bind(&query)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| KgError::Database(format!("FTS query failed: {}", e)))?;

        Ok(rows.into_iter().map(|r| {
            let rank = r.rank;
            (r.into_entity(), rank)
        }).collect())
    }

    /// Simple keyword search without rank.
    pub async fn search_simple(&self, keyword: &str, limit: usize) -> Result<Vec<Entity>, KgError> {
        let results = self.search(keyword, limit).await?;
        Ok(results.into_iter().map(|(e, _)| e).collect())
    }
}

#[derive(Debug, sqlx::FromRow)]
struct FtsSearchRow {
    id: String,
    name: String,
    entity_type: String,
    attributes_json: String,
    description: Option<String>,
    created_at: i64,
    updated_at: i64,
    version: i64,
    source: Option<String>,
    tags_json: String,
    aliases_json: String,
    rank: f64,
}

impl FtsSearchRow {
    fn into_entity(self) -> Entity {
        let attributes: HashMap<String, AttributeValue> =
            serde_json::from_str(&self.attributes_json).unwrap_or_default();
        let tags: Vec<String> = serde_json::from_str(&self.tags_json).unwrap_or_default();
        let aliases: Vec<String> = serde_json::from_str(&self.aliases_json).unwrap_or_default();

        Entity {
            id: self.id,
            name: self.name,
            entity_type: EntityType::from_str(&self.entity_type),
            attributes,
            description: self.description,
            created_at: self.created_at as u64,
            updated_at: self.updated_at as u64,
            version: self.version as u64,
            source: self.source,
            tags,
            aliases,
        }
    }
}
