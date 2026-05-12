use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt::Debug;

use crate::error::StoreError;
use crate::traits::{StoreLifecycle, VectorStore};
use crate::types::{MetadataFilter, SearchResult, StoreStats, VectorEntry};

/// sqlite-vec 向量存储实现
///
/// 特性：
/// - 零外部依赖（通过 sqlx + sqlite）
/// - 余弦距离搜索
/// - 元数据过滤通过 SQL WHERE 子句实现
/// - WAL 模式支持并发读
#[derive(Debug)]
pub struct SqliteVecStore {
    pool: sqlx::SqlitePool,
    dimensions: usize,
    table_name: String,
    max_capacity: Option<u64>,
}

impl SqliteVecStore {
    /// 创建新的 sqlite-vec 存储
    pub async fn new(
        path: &str,
        dimensions: usize,
        max_pool_size: Option<usize>,
    ) -> Result<Self, StoreError> {
        let pool_size = max_pool_size.unwrap_or(5);
        let conn_str = if path == ":memory:" {
            "sqlite::memory:".to_string()
        } else {
            format!("sqlite:{path}")
        };

        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(pool_size as u32)
            .connect(&conn_str)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))?;

        // 启用 WAL 模式
        sqlx::query("PRAGMA journal_mode=WAL")
            .execute(&pool)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))?;

        Ok(Self {
            pool,
            dimensions,
            table_name: "embeddings".into(),
            max_capacity: None,
        })
    }

    /// 设置表名
    pub fn with_table_name(mut self, name: &str) -> Self {
        self.table_name = name.to_string();
        self
    }

    /// 设置最大存储容量
    pub fn with_max_capacity(mut self, capacity: Option<u64>) -> Self {
        self.max_capacity = capacity;
        self
    }

    /// 清理过期数据
    pub async fn purge_expired(&self) -> Result<usize, StoreError> {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let result = sqlx::query(&format!(
            "DELETE FROM {} WHERE expires_at IS NOT NULL AND expires_at < ?",
            self.table_name
        ))
        .bind(now_ms as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| StoreError::Database(e.to_string()))?;

        Ok(result.rows_affected() as usize)
    }

    fn build_filter_clause(filter: &MetadataFilter) -> (String, Vec<String>) {
        match filter {
            MetadataFilter::Eq { key, value } => (
                format!("json_extract(metadata_json, '$.{key}') = ?"),
                vec![value.clone()],
            ),
            MetadataFilter::Ne { key, value } => (
                format!("json_extract(metadata_json, '$.{key}') != ?"),
                vec![value.clone()],
            ),
            MetadataFilter::In { key, values } => {
                let placeholders: Vec<String> = values.iter().map(|_| "?".to_string()).collect();
                (
                    format!(
                        "json_extract(metadata_json, '$.{key}') IN ({})",
                        placeholders.join(", ")
                    ),
                    values.clone(),
                )
            }
            MetadataFilter::NotIn { key, values } => {
                let placeholders: Vec<String> = values.iter().map(|_| "?".to_string()).collect();
                (
                    format!(
                        "json_extract(metadata_json, '$.{key}') NOT IN ({})",
                        placeholders.join(", ")
                    ),
                    values.clone(),
                )
            }
            MetadataFilter::Exists { key } => (
                format!("json_extract(metadata_json, '$.{key}') IS NOT NULL"),
                vec![],
            ),
            MetadataFilter::Contains { key, value } => (
                format!("json_extract(metadata_json, '$.{key}') LIKE ?"),
                vec![format!("%{value}%")],
            ),
            MetadataFilter::Range { key, min, max } => {
                let mut clauses = Vec::new();
                let mut params = Vec::new();
                if let Some(min_val) = min {
                    clauses.push(format!(
                        "CAST(json_extract(metadata_json, '$.{key}') AS REAL) >= ?"
                    ));
                    params.push(min_val.to_string());
                }
                if let Some(max_val) = max {
                    clauses.push(format!(
                        "CAST(json_extract(metadata_json, '$.{key}') AS REAL) <= ?"
                    ));
                    params.push(max_val.to_string());
                }
                (clauses.join(" AND "), params)
            }
            MetadataFilter::And(filters) => {
                let mut clauses = Vec::new();
                let mut all_params = Vec::new();
                for f in filters {
                    let (clause, mut params) = Self::build_filter_clause(f);
                    if !clause.is_empty() {
                        clauses.push(format!("({clause})"));
                        all_params.append(&mut params);
                    }
                }
                (clauses.join(" AND "), all_params)
            }
            MetadataFilter::Or(filters) => {
                let mut clauses = Vec::new();
                let mut all_params = Vec::new();
                for f in filters {
                    let (clause, mut params) = Self::build_filter_clause(f);
                    if !clause.is_empty() {
                        clauses.push(format!("({clause})"));
                        all_params.append(&mut params);
                    }
                }
                (clauses.join(" OR "), all_params)
            }
            MetadataFilter::Not(filter) => {
                let (inner, params) = Self::build_filter_clause(filter);
                (format!("NOT ({inner})"), params)
            }
        }
    }

    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }
        let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        dot / (norm_a * norm_b)
    }

    fn vector_to_blob(v: &[f32]) -> Vec<u8> {
        v.iter().flat_map(|f| f.to_le_bytes()).collect()
    }

    fn blob_to_vector(b: &[u8]) -> Vec<f32> {
        b.chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect()
    }
}

#[async_trait]
impl VectorStore for SqliteVecStore {
    async fn insert(&self, entry: VectorEntry) -> Result<(), StoreError> {
        self.insert_batch(vec![entry]).await
    }

    async fn insert_batch(&self, entries: Vec<VectorEntry>) -> Result<(), StoreError> {
        if entries.is_empty() {
            return Ok(());
        }

        // 检查维度
        for entry in &entries {
            if entry.vector.len() != self.dimensions {
                return Err(StoreError::DimensionMismatch {
                    expected: self.dimensions,
                    actual: entry.vector.len(),
                });
            }
        }

        for entry in entries {
            let vector_blob = Self::vector_to_blob(&entry.vector);
            let metadata_json = serde_json::to_string(&entry.metadata)
                .map_err(|e| StoreError::Serialization(e.to_string()))?;

            sqlx::query(&format!(
                "INSERT OR REPLACE INTO {} (id, content, metadata_json, channel, created_at, expires_at, embedding) VALUES (?, ?, ?, ?, ?, ?, ?)",
                self.table_name
            ))
            .bind(&entry.id)
            .bind(&entry.content)
            .bind(&metadata_json)
            .bind(&entry.channel)
            .bind(entry.created_at as i64)
            .bind(entry.expires_at.map(|t| t as i64))
            .bind(&vector_blob)
            .execute(&self.pool)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))?;
        }

        Ok(())
    }

    async fn search(&self, query: &[f32], limit: usize) -> Result<Vec<SearchResult>, StoreError> {
        if query.len() != self.dimensions {
            return Err(StoreError::DimensionMismatch {
                expected: self.dimensions,
                actual: query.len(),
            });
        }

        let rows = sqlx::query_as::<_, EmbeddingRow>(&format!(
            "SELECT id, content, metadata_json, channel, embedding FROM {}",
            self.table_name
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StoreError::Database(e.to_string()))?;

        let mut scored: Vec<(SearchResult, f32)> = rows
            .iter()
            .map(|row| {
                let vector = Self::blob_to_vector(&row.embedding);
                let similarity = Self::cosine_similarity(query, &vector);
                let metadata: HashMap<String, String> =
                    serde_json::from_str(&row.metadata_json).unwrap_or_default();

                (
                    SearchResult {
                        id: row.id.clone(),
                        score: similarity,
                        metadata,
                        content: row.content.clone(),
                        channel: row.channel.clone(),
                    },
                    similarity,
                )
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        Ok(scored.into_iter().map(|(r, _)| r).collect())
    }

    async fn search_with_filter(
        &self,
        query: &[f32],
        filter: &MetadataFilter,
        limit: usize,
    ) -> Result<Vec<SearchResult>, StoreError> {
        if query.len() != self.dimensions {
            return Err(StoreError::DimensionMismatch {
                expected: self.dimensions,
                actual: query.len(),
            });
        }

        let (filter_clause, params) = Self::build_filter_clause(filter);
        let sql = format!(
            "SELECT id, content, metadata_json, channel, embedding FROM {} WHERE {}",
            self.table_name, filter_clause
        );

        let mut query_builder = sqlx::query_as::<_, EmbeddingRow>(&sql);
        for param in &params {
            query_builder = query_builder.bind(param);
        }

        let rows = query_builder
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))?;

        let mut scored: Vec<(SearchResult, f32)> = rows
            .iter()
            .map(|row| {
                let vector = Self::blob_to_vector(&row.embedding);
                let similarity = Self::cosine_similarity(query, &vector);
                let metadata: HashMap<String, String> =
                    serde_json::from_str(&row.metadata_json).unwrap_or_default();

                (
                    SearchResult {
                        id: row.id.clone(),
                        score: similarity,
                        metadata,
                        content: row.content.clone(),
                        channel: row.channel.clone(),
                    },
                    similarity,
                )
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        Ok(scored.into_iter().map(|(r, _)| r).collect())
    }

    async fn delete(&self, ids: &[String]) -> Result<usize, StoreError> {
        if ids.is_empty() {
            return Ok(0);
        }
        let placeholders: Vec<String> = ids.iter().map(|_| "?".to_string()).collect();
        let sql = format!(
            "DELETE FROM {} WHERE id IN ({})",
            self.table_name,
            placeholders.join(", ")
        );

        let mut query = sqlx::query(&sql);
        for id in ids {
            query = query.bind(id);
        }

        let result = query
            .execute(&self.pool)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))?;

        Ok(result.rows_affected() as usize)
    }

    async fn delete_by_filter(&self, filter: &MetadataFilter) -> Result<usize, StoreError> {
        let (filter_clause, params) = Self::build_filter_clause(filter);
        let sql = format!("DELETE FROM {} WHERE {}", self.table_name, filter_clause);

        let mut query = sqlx::query(&sql);
        for param in &params {
            query = query.bind(param);
        }

        let result = query
            .execute(&self.pool)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))?;

        Ok(result.rows_affected() as usize)
    }

    async fn clear(&self) -> Result<(), StoreError> {
        sqlx::query(&format!("DELETE FROM {}", self.table_name))
            .execute(&self.pool)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))?;
        Ok(())
    }

    async fn count(&self) -> Result<usize, StoreError> {
        let (count,): (i64,) = sqlx::query_as(&format!(
            "SELECT COUNT(*) FROM {}",
            self.table_name
        ))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| StoreError::Database(e.to_string()))?;

        Ok(count as usize)
    }

    async fn rebuild_index(&self) -> Result<(), StoreError> {
        // sqlite-vec 不依赖传统索引，此操作仅做 WAL checkpoint
        sqlx::query("PRAGMA wal_checkpoint(FULL)")
            .execute(&self.pool)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))?;
        Ok(())
    }

    async fn stats(&self) -> Result<StoreStats, StoreError> {
        let count = self.count().await?;
        Ok(StoreStats {
            total_vectors: count,
            total_dimensions: self.dimensions,
            index_size_bytes: 0,
            data_size_bytes: 0,
            last_indexed_at: None,
        })
    }
}

#[async_trait]
impl StoreLifecycle for SqliteVecStore {
    async fn initialize(&self) -> Result<(), StoreError> {
        sqlx::query(&format!(
            "CREATE TABLE IF NOT EXISTS {} (
                id TEXT PRIMARY KEY,
                content TEXT,
                metadata_json TEXT,
                channel TEXT,
                created_at INTEGER NOT NULL,
                expires_at INTEGER,
                embedding BLOB NOT NULL
            )",
            self.table_name
        ))
        .execute(&self.pool)
        .await
        .map_err(|e| StoreError::Database(e.to_string()))?;

        Ok(())
    }

    async fn close(&self) -> Result<(), StoreError> {
        self.pool.close().await;
        Ok(())
    }

    async fn checkpoint(&self) -> Result<(), StoreError> {
        sqlx::query("PRAGMA wal_checkpoint(FULL)")
            .execute(&self.pool)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))?;
        Ok(())
    }

    async fn health_check(&self) -> Result<bool, StoreError> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(|e| StoreError::Database(e.to_string()))?;
        Ok(true)
    }
}

#[derive(Debug, sqlx::FromRow)]
struct EmbeddingRow {
    id: String,
    content: Option<String>,
    metadata_json: String,
    channel: Option<String>,
    embedding: Vec<u8>,
}
