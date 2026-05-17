use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};

/// Priority queue entry for Dijkstra shortest_path. Min-heap via PartialOrd override.
#[derive(PartialEq)]
struct PathCost(f32, String);

impl Eq for PathCost {}

impl PartialOrd for PathCost {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.0.partial_cmp(&self.0)
    }
}

impl Ord for PathCost {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use tracing::{debug, info};

use crate::config::KgConfig;
use crate::error::KgError;
use crate::store::sqlite_schema::{DDL, FTS_TRIGGERS};
use crate::traits::KnowledgeGraph;
use crate::types::attribute::AttributeValue;
use crate::types::confidence::Confidence;
use crate::types::consistency::{ConsistencyIssue, ConsistencyIssueType, IssueSeverity};
use crate::types::entity::{Entity, EntityType};
use crate::types::graph::{GraphStats, PathStep, SubGraph};
use crate::types::import::{ImportResult, MergeStrategy, UpsertResult};
use crate::types::provenance::Provenance;
use crate::types::query::{
    EntityPage, EntityQuery, RelationQuery,
};
use crate::types::relation::{Relation, WeightStrategy};

/// SQLite-backed knowledge graph implementation.
#[derive(Debug)]
pub struct SqliteKnowledgeGraph {
    pool: SqlitePool,
    #[allow(dead_code)]
    merge_strategy: MergeStrategy,
    weight_strategy: WeightStrategy,
    max_bfs_depth: u32,
    max_path_search: u32,
}

impl SqliteKnowledgeGraph {
    pub async fn new(path: &str, config: KgConfig) -> Result<Self, KgError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(config.storage.pool_size)
            .connect(&format!("sqlite:{}", path))
            .await
            .map_err(|e| KgError::Database(e.to_string()))?;

        sqlx::query("PRAGMA journal_mode=WAL")
            .execute(&pool)
            .await
            .map_err(|e| KgError::Database(e.to_string()))?;

        let this = Self {
            pool,
            merge_strategy: config.merge_strategy,
            weight_strategy: config.weight_strategy,
            max_bfs_depth: config.max_bfs_depth,
            max_path_search: config.max_path_search,
        };

        this.run_migrations().await?;
        Ok(this)
    }

    async fn run_migrations(&self) -> Result<(), KgError> {
        for stmt in DDL {
            sqlx::query(stmt)
                .execute(&self.pool)
                .await
                .map_err(|e| KgError::Database(format!("Migration failed: {}", e)))?;
        }
        for stmt in FTS_TRIGGERS {
            let _ = sqlx::query(stmt).execute(&self.pool).await;
        }
        debug!("sqlite schema migrations complete");
        Ok(())
    }
}

#[async_trait::async_trait]
impl KnowledgeGraph for SqliteKnowledgeGraph {
    // === Entity Operations ===

    async fn upsert_entity(&self, entity: Entity) -> Result<UpsertResult, KgError> {
        let existing: Option<EntityRow> = sqlx::query_as(
            "SELECT id, name, entity_type, attributes_json, description, created_at, updated_at,
                    version, source, tags_json, aliases_json
             FROM entities WHERE id = ?",
        )
        .bind(&entity.id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| KgError::Database(e.to_string()))?;

        let attrs_json =
            serde_json::to_string(&entity.attributes).map_err(|e| KgError::Serialization(e.to_string()))?;
        let tags_json =
            serde_json::to_string(&entity.tags).map_err(|e| KgError::Serialization(e.to_string()))?;
        let aliases_json =
            serde_json::to_string(&entity.aliases).map_err(|e| KgError::Serialization(e.to_string()))?;
        let entity_type = entity.entity_type.as_str();

        if let Some(row) = existing {
            let mut changed = Vec::new();
            let conflicts = Vec::new();

            if row.name != entity.name {
                changed.push("name".into());
            }
            if row.entity_type != entity_type {
                changed.push("entity_type".into());
            }

            if changed.is_empty() {
                return Ok(UpsertResult::Unchanged);
            }

            sqlx::query(
                "UPDATE entities SET name=?, entity_type=?, attributes_json=?, description=?,
                 updated_at=?, version=version+1, source=?, tags_json=?, aliases_json=?
                 WHERE id=?",
            )
            .bind(&entity.name)
            .bind(&entity_type)
            .bind(&attrs_json)
            .bind(&entity.description)
            .bind(current_epoch_ms() as i64)
            .bind(&entity.source)
            .bind(&tags_json)
            .bind(&aliases_json)
            .bind(&entity.id)
            .execute(&self.pool)
            .await
            .map_err(|e| KgError::Database(e.to_string()))?;

            Ok(UpsertResult::Updated { changed_fields: changed, conflicts })
        } else {
            sqlx::query(
                "INSERT INTO entities (id, name, entity_type, attributes_json, description,
                 created_at, updated_at, version, source, tags_json, aliases_json)
                 VALUES (?, ?, ?, ?, ?, ?, ?, 1, ?, ?, ?)",
            )
            .bind(&entity.id)
            .bind(&entity.name)
            .bind(&entity_type)
            .bind(&attrs_json)
            .bind(&entity.description)
            .bind(entity.created_at as i64)
            .bind(entity.updated_at as i64)
            .bind(&entity.source)
            .bind(&tags_json)
            .bind(&aliases_json)
            .execute(&self.pool)
            .await
            .map_err(|e| KgError::Database(e.to_string()))?;

            Ok(UpsertResult::Created)
        }
    }

    async fn get_entity(&self, id: &str) -> Result<Option<Entity>, KgError> {
        let row: Option<EntityRow> = sqlx::query_as(
            "SELECT id, name, entity_type, attributes_json, description, created_at, updated_at,
                    version, source, tags_json, aliases_json
             FROM entities WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| KgError::Database(e.to_string()))?;

        Ok(row.map(|r| r.into()))
    }

    async fn search_entities(&self, query: &EntityQuery) -> Result<EntityPage, KgError> {
        let use_fts = query.name_contains.is_some();

        let select_cols = "e.id, e.name, e.entity_type, e.attributes_json, e.description, \
            e.created_at, e.updated_at, e.version, e.source, e.tags_json, e.aliases_json";

        let mut sql = if use_fts {
            format!(
                "SELECT {} FROM entities e JOIN entities_fts fts ON e.rowid = fts.rowid WHERE entities_fts MATCH ?",
                select_cols
            )
        } else {
            format!("SELECT {} FROM entities e WHERE 1=1", select_cols)
        };
        let mut params: Vec<String> = Vec::new();

        if use_fts {
            // FTS5 query: append * for prefix matching
            let fts_query = format!("{}*", query.name_contains.as_ref().unwrap());
            params.push(fts_query);
        } else if let Some(ref name) = query.name_contains {
            params.push(format!("%{}%", name));
            sql.push_str(" AND e.name LIKE ?");
        }

        if let Some(ref aliases) = query.alias_contains {
            params.push(format!("%{}%", aliases));
            sql.push_str(" AND e.aliases_json LIKE ?");
        }
        if let Some(ref types) = query.entity_types {
            if !types.is_empty() {
                let type_strs: Vec<String> = types.iter().map(|t| t.as_str()).collect();
                let placeholders: Vec<String> = type_strs.iter().map(|_| "?".to_string()).collect();
                sql.push_str(&format!(" AND e.entity_type IN ({})", placeholders.join(",")));
                params.extend(type_strs);
            }
        }
        if let Some(ref source) = query.source {
            params.push(source.clone());
            sql.push_str(" AND e.source = ?");
        }
        // Tag filter
        if let Some(ref tag_filter) = query.tags {
            if !tag_filter.tags.is_empty() {
                match tag_filter.mode {
                    crate::types::query::TagFilterMode::Or => {
                        let tag_conditions: Vec<String> = tag_filter.tags.iter().map(|_| {
                            "e.tags_json LIKE '%' || ? || '%'".to_string()
                        }).collect();
                        sql.push_str(&format!(" AND ({})", tag_conditions.join(" OR ")));
                        params.extend(tag_filter.tags.iter().cloned());
                    }
                    crate::types::query::TagFilterMode::And => {
                        for tag in &tag_filter.tags {
                            sql.push_str(" AND e.tags_json LIKE '%' || ? || '%'");
                            params.push(tag.clone());
                        }
                    }
                }
            }
        }
        // Attribute filters
        for attr in &query.attribute_filters {
            let json_path = format!("$.{}", attr.key);
            match attr.operator {
                crate::types::query::FilterOperator::Eq => {
                    sql.push_str(" AND json_extract(e.attributes_json, ?) = ?");
                    params.push(json_path);
                    params.push(attr.value.clone());
                }
                crate::types::query::FilterOperator::Contains => {
                    sql.push_str(" AND json_extract(e.attributes_json, ?) LIKE '%' || ? || '%'");
                    params.push(json_path);
                    params.push(attr.value.clone());
                }
                _ => {
                    // Other operators: use JSON value comparison
                    sql.push_str(" AND json_extract(e.attributes_json, ?) = ?");
                    params.push(json_path);
                    params.push(attr.value.clone());
                }
            }
        }

        // Count
        let count_sql = sql.replace(
            &format!("SELECT {}", select_cols),
            "SELECT COUNT(*)",
        );

        let mut count_query = sqlx::query_scalar(&count_sql);
        for p in &params {
            count_query = count_query.bind(p);
        }
        let total: i64 = count_query
            .fetch_one(&self.pool)
            .await
            .map_err(|e| KgError::Database(e.to_string()))?;

        // Sort
        let order = match query.sort_by {
            Some(crate::types::query::EntitySortField::Name) => "e.name ASC",
            Some(crate::types::query::EntitySortField::CreatedAt) => "e.created_at DESC",
            Some(crate::types::query::EntitySortField::UpdatedAt) => "e.updated_at DESC",
            Some(crate::types::query::EntitySortField::EntityType) => "e.entity_type ASC",
            Some(crate::types::query::EntitySortField::RelationCount) => "e.updated_at DESC",
            None => {
                if use_fts { "ORDER BY rank" } else { "ORDER BY e.updated_at DESC" }
            }
        };
        sql.push_str(&format!(" {} LIMIT ? OFFSET ?", order));

        let mut fetch_query = sqlx::query_as::<_, EntityRow>(&sql);
        for p in &params {
            fetch_query = fetch_query.bind(p);
        }
        fetch_query = fetch_query
            .bind(query.page.limit as i64)
            .bind(query.page.offset as i64);

        let rows: Vec<EntityRow> = fetch_query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| KgError::Database(e.to_string()))?;

        let total = total as usize;
        let items: Vec<Entity> = rows.into_iter().map(|r| r.into()).collect();
        let has_more = query.page.offset + query.page.limit < total;

        Ok(EntityPage { items, total, has_more })
    }

    async fn delete_entity(&self, id: &str) -> Result<usize, KgError> {
        let mut txn = self.pool.begin().await.map_err(|e| KgError::Database(e.to_string()))?;

        let outcome = {
            let conn = std::ops::DerefMut::deref_mut(&mut txn);

            let relation_count: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM relations WHERE source_id = ? OR target_id = ?",
            )
            .bind(id)
            .bind(id)
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| KgError::Database(e.to_string()))?;

            sqlx::query("DELETE FROM relations WHERE source_id = ? OR target_id = ?")
                .bind(id)
                .bind(id)
                .execute(&mut *conn)
                .await
                .map_err(|e| KgError::Database(e.to_string()))?;

            sqlx::query("DELETE FROM entities WHERE id = ?")
                .bind(id)
                .execute(&mut *conn)
                .await
                .map_err(|e| KgError::Database(e.to_string()))?;

            Ok::<usize, KgError>(relation_count.0 as usize)
        };

        match outcome {
            Ok(count) => {
                txn.commit().await.map_err(|e| KgError::Database(e.to_string()))?;
                Ok(count)
            }
            Err(e) => {
                let _ = txn.rollback().await;
                Err(e)
            }
        }
    }

    async fn get_entities_batch(&self, ids: &[&str]) -> Result<Vec<Entity>, KgError> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let placeholders: Vec<String> = ids.iter().map(|_| "?".to_string()).collect();
        let sql = format!(
            "SELECT id, name, entity_type, attributes_json, description, created_at, updated_at,
                    version, source, tags_json, aliases_json
             FROM entities WHERE id IN ({})",
            placeholders.join(",")
        );

        let mut query = sqlx::query_as::<_, EntityRow>(&sql);
        for id in ids {
            query = query.bind(id);
        }

        let rows: Vec<EntityRow> = query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| KgError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    // === Relation Operations ===

    async fn upsert_relation(&self, relation: Relation) -> Result<UpsertResult, KgError> {
        let existing: Option<RelationRow> = sqlx::query_as(
            "SELECT id, source_id, target_id, relation_type, properties_json, confidence,
                    provenance_json, valid_from, valid_to, created_at, weight
             FROM relations WHERE id = ?",
        )
        .bind(&relation.id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| KgError::Database(e.to_string()))?;

        let props_json = serde_json::to_string(&relation.properties)
            .map_err(|e| KgError::Serialization(e.to_string()))?;
        let provenance_json = relation
            .provenance
            .as_ref()
            .map(|p| serde_json::to_string(p))
            .transpose()
            .map_err(|e| KgError::Serialization(e.to_string()))?;

        if existing.is_some() {
            sqlx::query(
                "UPDATE relations SET relation_type=?, properties_json=?, confidence=?,
                 provenance_json=?, valid_from=?, valid_to=?, weight=?
                 WHERE id=?",
            )
            .bind(&relation.relation_type)
            .bind(&props_json)
            .bind(relation.confidence.as_f32())
            .bind(&provenance_json)
            .bind(relation.valid_from.map(|v| v as i64))
            .bind(relation.valid_to.map(|v| v as i64))
            .bind(relation.weight)
            .bind(&relation.id)
            .execute(&self.pool)
            .await
            .map_err(|e| KgError::Database(e.to_string()))?;

            Ok(UpsertResult::Updated {
                changed_fields: vec!["relation_type".into()],
                conflicts: vec![],
            })
        } else {
            sqlx::query(
                "INSERT INTO relations (id, source_id, target_id, relation_type, properties_json,
                 confidence, provenance_json, valid_from, valid_to, created_at, weight)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&relation.id)
            .bind(&relation.source_id)
            .bind(&relation.target_id)
            .bind(&relation.relation_type)
            .bind(&props_json)
            .bind(relation.confidence.as_f32())
            .bind(&provenance_json)
            .bind(relation.valid_from.map(|v| v as i64))
            .bind(relation.valid_to.map(|v| v as i64))
            .bind(relation.created_at as i64)
            .bind(relation.weight)
            .execute(&self.pool)
            .await
            .map_err(|e| KgError::Database(e.to_string()))?;

            Ok(UpsertResult::Created)
        }
    }

    async fn get_relations(&self, entity_id: &str) -> Result<Vec<Relation>, KgError> {
        let rows: Vec<RelationRow> = sqlx::query_as(
            "SELECT id, source_id, target_id, relation_type, properties_json, confidence,
                    provenance_json, valid_from, valid_to, created_at, weight
             FROM relations WHERE source_id = ? OR target_id = ?",
        )
        .bind(entity_id)
        .bind(entity_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| KgError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn query_relations(&self, query: &RelationQuery) -> Result<Vec<Relation>, KgError> {
        let mut sql = String::from(
            "SELECT id, source_id, target_id, relation_type, properties_json, confidence,
                    provenance_json, valid_from, valid_to, created_at, weight
             FROM relations WHERE 1=1",
        );
        let mut params: Vec<String> = Vec::new();

        if let Some(ref sid) = query.source_id {
            params.push(sid.clone());
            sql.push_str(" AND source_id = ?");
        }
        if let Some(ref tid) = query.target_id {
            params.push(tid.clone());
            sql.push_str(" AND target_id = ?");
        }
        if let Some(ref eid) = query.entity_id {
            params.push(eid.clone());
            params.push(eid.clone());
            sql.push_str(" AND (source_id = ? OR target_id = ?)");
        }
        if let Some(ref rt) = query.relation_type {
            params.push(rt.clone());
            sql.push_str(" AND relation_type = ?");
        }
        if let Some(ref rts) = query.relation_types {
            if !rts.is_empty() {
                let placeholders: Vec<String> = rts.iter().map(|_| "?".to_string()).collect();
                sql.push_str(&format!(" AND relation_type IN ({})", placeholders.join(",")));
                params.extend(rts.iter().cloned());
            }
        }
        if let Some(ref min_conf) = query.min_confidence {
            sql.push_str(" AND confidence >= ?");
            params.push(min_conf.as_f32().to_string());
        }
        if let Some(valid_at) = query.valid_at {
            sql.push_str(" AND (valid_from IS NULL OR valid_from <= ?) AND (valid_to IS NULL OR valid_to >= ?)");
            params.push(valid_at.to_string());
            params.push(valid_at.to_string());
        }

        sql.push_str(" LIMIT ? OFFSET ?");

        let mut fetch_query = sqlx::query_as::<_, RelationRow>(&sql);
        for p in &params {
            fetch_query = fetch_query.bind(p);
        }
        fetch_query = fetch_query
            .bind(query.page.limit as i64)
            .bind(query.page.offset as i64);

        let rows: Vec<RelationRow> = fetch_query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| KgError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn delete_relation(&self, id: &str) -> Result<(), KgError> {
        let result = sqlx::query("DELETE FROM relations WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| KgError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(KgError::RelationNotFound(id.to_string()));
        }
        Ok(())
    }

    // === Graph Traversal ===

    async fn get_neighbors(&self, entity_id: &str, depth: u32) -> Result<SubGraph, KgError> {
        if depth > self.max_bfs_depth {
            return Err(KgError::MaxDepthExceeded {
                depth,
                max: self.max_bfs_depth,
            });
        }

        let center = self
            .get_entity(entity_id)
            .await?
            .ok_or_else(|| KgError::EntityNotFound(entity_id.to_string()))?;

        let mut visited_entities: HashMap<String, Entity> = HashMap::new();
        let mut visited_relations: Vec<Relation> = Vec::new();
        let mut queue: VecDeque<(String, u32)> = VecDeque::new();

        visited_entities.insert(entity_id.to_string(), center.clone());
        queue.push_back((entity_id.to_string(), 0));

        while let Some((current_id, current_depth)) = queue.pop_front() {
            if current_depth >= depth {
                continue;
            }

            // Get all relations for the current entity
            let relations = self.get_relations(&current_id).await?;
            for rel in relations {
                let neighbor_id = if rel.source_id == current_id {
                    rel.target_id.clone()
                } else {
                    rel.source_id.clone()
                };

                visited_relations.push(rel);

                if !visited_entities.contains_key(&neighbor_id) {
                    if let Some(entity) = self.get_entity(&neighbor_id).await? {
                        visited_entities.insert(neighbor_id.clone(), entity);
                        queue.push_back((neighbor_id, current_depth + 1));
                    }
                }
            }
        }

        let entities: Vec<Entity> = visited_entities
            .into_iter()
            .filter(|(id, _)| id != entity_id)
            .map(|(_, e)| e)
            .collect();

        Ok(SubGraph {
            center,
            entities,
            relations: visited_relations,
        })
    }

    async fn shortest_path(
        &self,
        from: &str,
        to: &str,
    ) -> Result<Option<Vec<PathStep>>, KgError> {
        if from == to {
            return Ok(Some(vec![]));
        }

        // Load all entities and relations into memory for path finding
        let entity_rows: Vec<EntityRow> = sqlx::query_as(
            "SELECT id, name, entity_type, attributes_json, description, created_at, updated_at,
                    version, source, tags_json, aliases_json FROM entities",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| KgError::Database(e.to_string()))?;

        let entities: HashMap<String, Entity> =
            entity_rows.into_iter().map(|r| (r.id.clone(), r.into())).collect();

        let relation_rows: Vec<RelationRow> = sqlx::query_as(
            "SELECT id, source_id, target_id, relation_type, properties_json, confidence,
                    provenance_json, valid_from, valid_to, created_at, weight FROM relations",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| KgError::Database(e.to_string()))?;

        let relations: Vec<Relation> = relation_rows.into_iter().map(|r| r.into()).collect();

        // Build adjacency lists
        let mut adj: HashMap<String, Vec<(String, Relation)>> = HashMap::new();
        for rel in &relations {
            adj.entry(rel.source_id.clone())
                .or_default()
                .push((rel.target_id.clone(), rel.clone()));
            adj.entry(rel.target_id.clone())
                .or_default()
                .push((rel.source_id.clone(), rel.clone()));
        }

        let mut dist: HashMap<String, f32> = HashMap::new();
        let mut prev: HashMap<String, (String, Relation)> = HashMap::new();
        let initial_dist = f32::MAX;

        for id in entities.keys() {
            dist.insert(id.clone(), initial_dist);
        }
        dist.insert(from.to_string(), 0.0);

        let mut queue: BinaryHeap<PathCost> = BinaryHeap::new();
        queue.push(PathCost(0.0, from.to_string()));

        while let Some(PathCost(_d, u)) = queue.pop() {
            if let Some(neighbors) = adj.get(&u) {
                for (v, rel) in neighbors {
                    let weight = self.weight_strategy.relation_cost(rel);
                    let alt = dist.get(&u).copied().unwrap_or(initial_dist) + weight;
                    if alt < dist.get(v).copied().unwrap_or(initial_dist) {
                        dist.insert(v.clone(), alt);
                        prev.insert(v.clone(), (u.clone(), rel.clone()));
                        queue.push(PathCost(alt, v.clone()));
                    }
                }
            }
        }

        if !prev.contains_key(to) && from != to {
            return Ok(None);
        }

        // Reconstruct path
        let mut path = Vec::new();
        let mut current = to.to_string();
        while current != from {
            if let Some((prev_node, rel)) = prev.get(&current) {
                let entity = entities.get(&current).cloned().unwrap();
                path.push(PathStep { entity, relation: rel.clone() });
                current = prev_node.clone();
            } else {
                break;
            }
        }
        // Add the starting entity
        path.reverse();

        Ok(Some(path))
    }

    async fn all_paths(
        &self,
        from: &str,
        to: &str,
        max_depth: u32,
    ) -> Result<Vec<Vec<PathStep>>, KgError> {
        if max_depth > self.max_path_search {
            return Err(KgError::MaxDepthExceeded {
                depth: max_depth,
                max: self.max_path_search,
            });
        }

        let entity_rows: Vec<EntityRow> = sqlx::query_as(
            "SELECT id, name, entity_type, attributes_json, description, created_at, updated_at,
                    version, source, tags_json, aliases_json FROM entities",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| KgError::Database(e.to_string()))?;

        let entities: HashMap<String, Entity> =
            entity_rows.into_iter().map(|r| (r.id.clone(), r.into())).collect();

        let relation_rows: Vec<RelationRow> = sqlx::query_as(
            "SELECT id, source_id, target_id, relation_type, properties_json, confidence,
                    provenance_json, valid_from, valid_to, created_at, weight FROM relations",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| KgError::Database(e.to_string()))?;

        let relations: Vec<Relation> = relation_rows.into_iter().map(|r| r.into()).collect();

        // Build adjacency lists
        let mut adj: HashMap<String, Vec<(String, Relation)>> = HashMap::new();
        for rel in &relations {
            adj.entry(rel.source_id.clone())
                .or_default()
                .push((rel.target_id.clone(), rel.clone()));
            adj.entry(rel.target_id.clone())
                .or_default()
                .push((rel.source_id.clone(), rel.clone()));
        }

        let mut all_paths: Vec<Vec<PathStep>> = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut current_path: Vec<PathStep> = Vec::new();

        dfs_all_paths(
            from,
            to,
            max_depth,
            &entities,
            &adj,
            &mut visited,
            &mut current_path,
            &mut all_paths,
        );

        all_paths.sort_by(|a, b| {
            let a_cost: f32 = a.iter().map(|step| self.weight_strategy.relation_cost(&step.relation)).sum();
            let b_cost: f32 = b.iter().map(|step| self.weight_strategy.relation_cost(&step.relation)).sum();
            a_cost.partial_cmp(&b_cost).unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(all_paths)
    }

    // === Batch Operations ===

    async fn batch_import(
        &self,
        entities: Vec<Entity>,
        relations: Vec<Relation>,
    ) -> Result<ImportResult, KgError> {
        let mut txn = self.pool.begin().await.map_err(|e| KgError::Database(e.to_string()))?;

        // Scoped to allow fallback to rollback after conn is dropped
        let outcome = {
            let conn = std::ops::DerefMut::deref_mut(&mut txn);
            let mut result = ImportResult::default();

            for entity in &entities {
                match batch_upsert_entity(conn, entity).await {
                    Ok(UpsertResult::Created) => result.entities_created += 1,
                    Ok(UpsertResult::Updated { conflicts, .. }) => {
                        result.entities_updated += 1;
                        result.conflicts.extend(conflicts);
                    }
                    Ok(UpsertResult::Unchanged) => result.entities_skipped += 1,
                    Err(e) => return Err(e),
                }
            }

            for relation in &relations {
                match batch_upsert_relation(conn, relation).await {
                    Ok(UpsertResult::Created) => result.relations_created += 1,
                    Ok(UpsertResult::Updated { .. }) => result.relations_updated += 1,
                    Ok(UpsertResult::Unchanged) => {}
                    Err(e) => return Err(e),
                }
            }

            Ok(result)
        };

        match outcome {
            Ok(result) => {
                txn.commit().await.map_err(|e| KgError::Database(e.to_string()))?;
                info!(
                    entities_created = %result.entities_created,
                    entities_updated = %result.entities_updated,
                    relations_created = %result.relations_created,
                    "batch import completed"
                );
                Ok(result)
            }
            Err(e) => {
                let _ = txn.rollback().await;
                Err(e)
            }
        }
    }

    // === Consistency ===

    async fn check_consistency(&self) -> Result<Vec<ConsistencyIssue>, KgError> {
        let mut issues = Vec::new();

        // Check 1: Orphan relations
        let orphans: Vec<OrphanRelationRow> = sqlx::query_as(
            "SELECT r.id, r.source_id, r.target_id
             FROM relations r
             LEFT JOIN entities e1 ON r.source_id = e1.id
             LEFT JOIN entities e2 ON r.target_id = e2.id
             WHERE e1.id IS NULL OR e2.id IS NULL",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| KgError::Database(e.to_string()))?;

        for o in orphans {
            issues.push(ConsistencyIssue {
                severity: IssueSeverity::Error,
                issue_type: ConsistencyIssueType::OrphanRelation,
                description: format!("Relation {} references a non-existent entity", o.id),
                related_entities: vec![o.source_id, o.target_id],
                related_relations: vec![o.id],
            });
        }

        // Check 2: Self-referencing
        let self_refs: Vec<RelationRow> = sqlx::query_as(
            "SELECT id, source_id, target_id, relation_type, properties_json, confidence,
                    provenance_json, valid_from, valid_to, created_at, weight
             FROM relations WHERE source_id = target_id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| KgError::Database(e.to_string()))?;

        for rel in self_refs {
            issues.push(ConsistencyIssue {
                severity: IssueSeverity::Warning,
                issue_type: ConsistencyIssueType::SelfReferencing,
                description: format!("Relation {} self-references entity {}", rel.id, rel.source_id),
                related_entities: vec![rel.source_id],
                related_relations: vec![rel.id],
            });
        }

        // Check 3: Orphan entities
        let orphan_entities: Vec<(String, String)> = sqlx::query_as(
            "SELECT e.id, e.name FROM entities e
             WHERE e.id NOT IN (SELECT source_id FROM relations)
               AND e.id NOT IN (SELECT target_id FROM relations)",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| KgError::Database(e.to_string()))?;

        for (id, name) in orphan_entities {
            issues.push(ConsistencyIssue {
                severity: IssueSeverity::Info,
                issue_type: ConsistencyIssueType::OrphanEntity,
                description: format!("Entity {} ({}) has no relations", name, id),
                related_entities: vec![id],
                related_relations: vec![],
            });
        }

        // Check 4: Expired relations
        let now = current_epoch_ms();
        let expired: Vec<RelationRow> = sqlx::query_as(
            "SELECT id, source_id, target_id, relation_type, properties_json, confidence,
                    provenance_json, valid_from, valid_to, created_at, weight
             FROM relations WHERE valid_to IS NOT NULL AND valid_to < ?",
        )
        .bind(now as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| KgError::Database(e.to_string()))?;

        for rel in expired {
            issues.push(ConsistencyIssue {
                severity: IssueSeverity::Warning,
                issue_type: ConsistencyIssueType::ExpiredRelation,
                description: format!("Relation {} has expired (valid_to < now)", rel.id),
                related_entities: vec![rel.source_id, rel.target_id],
                related_relations: vec![rel.id],
            });
        }

        Ok(issues)
    }

    // === Statistics ===

    async fn stats(&self) -> Result<GraphStats, KgError> {
        let total_entities: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM entities")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| KgError::Database(e.to_string()))?;

        let total_relations: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM relations")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| KgError::Database(e.to_string()))?;

        let entity_types: Vec<(String, i64)> = sqlx::query_as(
            "SELECT entity_type, COUNT(*) as cnt FROM entities GROUP BY entity_type",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| KgError::Database(e.to_string()))?;

        let relation_types: Vec<(String, i64)> = sqlx::query_as(
            "SELECT relation_type, COUNT(*) as cnt FROM relations GROUP BY relation_type",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| KgError::Database(e.to_string()))?;

        // Calculate degrees
        let degrees: Vec<(i64,)> = sqlx::query_as(
            "SELECT COUNT(*) FROM relations GROUP BY source_id
             UNION ALL SELECT COUNT(*) FROM relations GROUP BY target_id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| KgError::Database(e.to_string()))?;

        let degree_values: Vec<usize> = degrees.into_iter().map(|d| d.0 as usize).collect();
        let avg_degree = if degree_values.is_empty() {
            0.0
        } else {
            degree_values.iter().sum::<usize>() as f64 / degree_values.len() as f64
        };
        let max_degree = degree_values.iter().max().copied().unwrap_or(0);

        // Orphan entities
        let orphan_entities: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM entities e
             WHERE e.id NOT IN (SELECT source_id FROM relations)
               AND e.id NOT IN (SELECT target_id FROM relations)",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| KgError::Database(e.to_string()))?;

        // DB size
        let db_size: (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(pgsize), 0) FROM dbstat",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| KgError::Database(e.to_string()))?;

        Ok(GraphStats {
            total_entities: total_entities.0 as usize,
            total_relations: total_relations.0 as usize,
            entity_types: entity_types.into_iter().map(|(k, v)| (k, v as usize)).collect(),
            relation_types: relation_types.into_iter().map(|(k, v)| (k, v as usize)).collect(),
            avg_degree,
            max_degree,
            orphan_entities: orphan_entities.0 as usize,
            db_size_bytes: db_size.0 as u64,
        })
    }
}

// === DFS helper ===

#[allow(clippy::too_many_arguments)]
fn dfs_all_paths(
    current: &str,
    target: &str,
    max_depth: u32,
    entities: &HashMap<String, Entity>,
    adj: &HashMap<String, Vec<(String, Relation)>>,
    visited: &mut HashSet<String>,
    current_path: &mut Vec<PathStep>,
    all_paths: &mut Vec<Vec<PathStep>>,
) {
    if current == target {
        all_paths.push(current_path.clone());
        return;
    }
    if current_path.len() >= max_depth as usize {
        return;
    }
    visited.insert(current.to_string());

    if let Some(neighbors) = adj.get(current) {
        for (neighbor, rel) in neighbors {
            if visited.contains(neighbor.as_str()) {
                continue;
            }
            if let Some(entity) = entities.get(neighbor).cloned() {
                current_path.push(PathStep {
                    entity,
                    relation: rel.clone(),
                });
                dfs_all_paths(
                    neighbor, target, max_depth, entities, adj,
                    visited, current_path, all_paths,
                );
                current_path.pop();
            }
        }
    }

    visited.remove(current);
}

// === Batch helpers (operate on a &mut SqliteConnection within a transaction) ===

async fn batch_upsert_entity(
    conn: &mut sqlx::SqliteConnection,
    entity: &Entity,
) -> Result<UpsertResult, KgError> {
    let existing: Option<EntityRow> = sqlx::query_as(
        "SELECT id, name, entity_type, attributes_json, description, created_at, updated_at,
                version, source, tags_json, aliases_json
         FROM entities WHERE id = ?",
    )
    .bind(&entity.id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(|e| KgError::Database(e.to_string()))?;

    let attrs_json =
        serde_json::to_string(&entity.attributes).map_err(|e| KgError::Serialization(e.to_string()))?;
    let tags_json =
        serde_json::to_string(&entity.tags).map_err(|e| KgError::Serialization(e.to_string()))?;
    let aliases_json =
        serde_json::to_string(&entity.aliases).map_err(|e| KgError::Serialization(e.to_string()))?;
    let entity_type = entity.entity_type.as_str();

    if let Some(row) = existing {
        let mut changed = Vec::new();
        let conflicts = Vec::new();

        if row.name != entity.name {
            changed.push("name".into());
        }
        if row.entity_type != entity_type {
            changed.push("entity_type".into());
        }

        if changed.is_empty() {
            return Ok(UpsertResult::Unchanged);
        }

        sqlx::query(
            "UPDATE entities SET name=?, entity_type=?, attributes_json=?, description=?,
             updated_at=?, version=version+1, source=?, tags_json=?, aliases_json=?
             WHERE id=?",
        )
        .bind(&entity.name)
        .bind(&entity_type)
        .bind(&attrs_json)
        .bind(&entity.description)
        .bind(current_epoch_ms() as i64)
        .bind(&entity.source)
        .bind(&tags_json)
        .bind(&aliases_json)
        .bind(&entity.id)
        .execute(&mut *conn)
        .await
        .map_err(|e| KgError::Database(e.to_string()))?;

        Ok(UpsertResult::Updated { changed_fields: changed, conflicts })
    } else {
        sqlx::query(
            "INSERT INTO entities (id, name, entity_type, attributes_json, description,
             created_at, updated_at, version, source, tags_json, aliases_json)
             VALUES (?, ?, ?, ?, ?, ?, ?, 1, ?, ?, ?)",
        )
        .bind(&entity.id)
        .bind(&entity.name)
        .bind(&entity_type)
        .bind(&attrs_json)
        .bind(&entity.description)
        .bind(entity.created_at as i64)
        .bind(entity.updated_at as i64)
        .bind(&entity.source)
        .bind(&tags_json)
        .bind(&aliases_json)
        .execute(&mut *conn)
        .await
        .map_err(|e| KgError::Database(e.to_string()))?;

        Ok(UpsertResult::Created)
    }
}

async fn batch_upsert_relation(
    conn: &mut sqlx::SqliteConnection,
    relation: &Relation,
) -> Result<UpsertResult, KgError> {
    let existing: Option<RelationRow> = sqlx::query_as(
        "SELECT id, source_id, target_id, relation_type, properties_json, confidence,
                provenance_json, valid_from, valid_to, created_at, weight
         FROM relations WHERE id = ?",
    )
    .bind(&relation.id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(|e| KgError::Database(e.to_string()))?;

    let props_json = serde_json::to_string(&relation.properties)
        .map_err(|e| KgError::Serialization(e.to_string()))?;
    let provenance_json = relation
        .provenance
        .as_ref()
        .map(|p| serde_json::to_string(p))
        .transpose()
        .map_err(|e| KgError::Serialization(e.to_string()))?;

    if existing.is_some() {
        sqlx::query(
            "UPDATE relations SET relation_type=?, properties_json=?, confidence=?,
             provenance_json=?, valid_from=?, valid_to=?, weight=?
             WHERE id=?",
        )
        .bind(&relation.relation_type)
        .bind(&props_json)
        .bind(relation.confidence.as_f32())
        .bind(&provenance_json)
        .bind(relation.valid_from.map(|v| v as i64))
        .bind(relation.valid_to.map(|v| v as i64))
        .bind(relation.weight)
        .bind(&relation.id)
        .execute(&mut *conn)
        .await
        .map_err(|e| KgError::Database(e.to_string()))?;

        Ok(UpsertResult::Updated {
            changed_fields: vec!["relation_type".into()],
            conflicts: vec![],
        })
    } else {
        sqlx::query(
            "INSERT INTO relations (id, source_id, target_id, relation_type, properties_json,
             confidence, provenance_json, valid_from, valid_to, created_at, weight)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&relation.id)
        .bind(&relation.source_id)
        .bind(&relation.target_id)
        .bind(&relation.relation_type)
        .bind(&props_json)
        .bind(relation.confidence.as_f32())
        .bind(&provenance_json)
        .bind(relation.valid_from.map(|v| v as i64))
        .bind(relation.valid_to.map(|v| v as i64))
        .bind(relation.created_at as i64)
        .bind(relation.weight)
        .execute(&mut *conn)
        .await
        .map_err(|e| KgError::Database(e.to_string()))?;

        Ok(UpsertResult::Created)
    }
}

// === Row types ===

#[derive(Debug, sqlx::FromRow)]
struct EntityRow {
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
}

impl From<EntityRow> for Entity {
    fn from(r: EntityRow) -> Self {
        let attributes: HashMap<String, AttributeValue> =
            serde_json::from_str(&r.attributes_json).unwrap_or_default();
        let tags: Vec<String> = serde_json::from_str(&r.tags_json).unwrap_or_default();
        let aliases: Vec<String> = serde_json::from_str(&r.aliases_json).unwrap_or_default();

        Self {
            id: r.id,
            name: r.name,
            entity_type: EntityType::from_str(&r.entity_type),
            attributes,
            description: r.description,
            created_at: r.created_at as u64,
            updated_at: r.updated_at as u64,
            version: r.version as u64,
            source: r.source,
            tags,
            aliases,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct RelationRow {
    id: String,
    source_id: String,
    target_id: String,
    relation_type: String,
    properties_json: String,
    confidence: f32,
    provenance_json: Option<String>,
    valid_from: Option<i64>,
    valid_to: Option<i64>,
    created_at: i64,
    weight: Option<f32>,
}

impl From<RelationRow> for Relation {
    fn from(r: RelationRow) -> Self {
        let properties: HashMap<String, String> =
            serde_json::from_str(&r.properties_json).unwrap_or_default();
        let provenance: Option<Provenance> = r
            .provenance_json
            .and_then(|j| serde_json::from_str(&j).ok());

        Self {
            id: r.id,
            source_id: r.source_id,
            target_id: r.target_id,
            relation_type: r.relation_type,
            properties,
            confidence: Confidence::from_f32(r.confidence),
            provenance,
            valid_from: r.valid_from.map(|v| v as u64),
            valid_to: r.valid_to.map(|v| v as u64),
            created_at: r.created_at as u64,
            weight: r.weight,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct OrphanRelationRow {
    id: String,
    source_id: String,
    target_id: String,
}

// === Utility ===

fn current_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
