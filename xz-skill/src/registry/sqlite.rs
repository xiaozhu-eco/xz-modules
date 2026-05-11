#[cfg(feature = "sqlite-registry")]
use async_trait::async_trait;
#[cfg(feature = "sqlite-registry")]
use sqlx::SqlitePool;

#[cfg(feature = "sqlite-registry")]
use crate::error::SkillError;
#[cfg(feature = "sqlite-registry")]
use crate::traits::SkillRegistry;
#[cfg(feature = "sqlite-registry")]
use crate::types::filter::SkillFilter;
use crate::types::output::SkillSummary;
#[cfg(feature = "sqlite-registry")]
use crate::types::skill::{Skill, UpsertResult};

#[cfg(feature = "sqlite-registry")]
#[derive(Debug)]
pub struct SqliteSkillRegistry {
    pool: SqlitePool,
}

#[cfg(feature = "sqlite-registry")]
impl SqliteSkillRegistry {
    pub async fn new(database_url: &str) -> Result<Self, SkillError> {
        let pool = SqlitePool::connect(database_url)
            .await
            .map_err(|e| SkillError::ConfigValidation(e.to_string()))?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS skills (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                author TEXT NOT NULL DEFAULT '',
                prompt TEXT NOT NULL DEFAULT '',
                tools_json TEXT NOT NULL DEFAULT '[]',
                permissions_json TEXT NOT NULL DEFAULT '[]',
                config_schema_json TEXT,
                default_config_json TEXT,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                min_agent_version TEXT
            )
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| SkillError::ConfigValidation(e.to_string()))?;

        Ok(Self { pool })
    }
}

#[cfg(feature = "sqlite-registry")]
#[async_trait]
impl SkillRegistry for SqliteSkillRegistry {
    async fn register(&self, skill: Skill) -> Result<UpsertResult, SkillError> {
        let tools_json =
            serde_json::to_string(&skill.tools).map_err(|e| SkillError::Yaml(e.to_string()))?;
        let permissions_json = serde_json::to_string(&skill.permissions)
            .map_err(|e| SkillError::Yaml(e.to_string()))?;
        let config_schema_json = skill
            .config_schema
            .as_ref()
            .map(|v| v.to_string());
        let default_config_json = skill
            .default_config
            .as_ref()
            .map(|v| v.to_string());

        let existing = sqlx::query_scalar::<_, String>("SELECT id FROM skills WHERE id = ?")
            .bind(&skill.id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| SkillError::ConfigValidation(e.to_string()))?;

        if existing.is_some() {
            sqlx::query(
                "UPDATE skills SET name=?, version=?, description=?, prompt=?, tools_json=?,
                 permissions_json=?, enabled=?, updated_at=? WHERE id=?",
            )
            .bind(&skill.name)
            .bind(&skill.version)
            .bind(&skill.description)
            .bind(&skill.prompt)
            .bind(&tools_json)
            .bind(&permissions_json)
            .bind(skill.enabled as i32)
            .bind(skill.updated_at as i64)
            .bind(&skill.id)
            .execute(&self.pool)
            .await
            .map_err(|e| SkillError::ConfigValidation(e.to_string()))?;
            Ok(UpsertResult::Updated {
                changed_fields: vec!["*".into()],
            })
        } else {
            sqlx::query(
                "INSERT INTO skills (id, name, version, description, author, prompt, tools_json,
                 permissions_json, config_schema_json, default_config_json, enabled,
                 created_at, updated_at, min_agent_version)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&skill.id)
            .bind(&skill.name)
            .bind(&skill.version)
            .bind(&skill.description)
            .bind(&skill.author)
            .bind(&skill.prompt)
            .bind(&tools_json)
            .bind(&permissions_json)
            .bind(&config_schema_json)
            .bind(&default_config_json)
            .bind(skill.enabled as i32)
            .bind(skill.created_at as i64)
            .bind(skill.updated_at as i64)
            .bind(&skill.min_agent_version)
            .execute(&self.pool)
            .await
            .map_err(|e| SkillError::ConfigValidation(e.to_string()))?;
            Ok(UpsertResult::Created)
        }
    }

    async fn unregister(&self, id: &str) -> Result<(), SkillError> {
        let rows = sqlx::query("DELETE FROM skills WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| SkillError::ConfigValidation(e.to_string()))?
            .rows_affected();
        if rows == 0 {
            return Err(SkillError::NotFound(id.to_string()));
        }
        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Option<Skill>, SkillError> {
        let row = sqlx::query_as::<_, SkillRow>(
            "SELECT * FROM skills WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| SkillError::ConfigValidation(e.to_string()))?;

        row.map(|r| r.into_skill()).transpose()
    }

    async fn list(&self, filter: &SkillFilter) -> Result<Vec<SkillSummary>, SkillError> {
        let mut query = String::from(
            "SELECT id, name, version, description, author, enabled, tools_json FROM skills WHERE 1=1",
        );
        if filter.enabled_only {
            query.push_str(" AND enabled = 1");
        }
        if let Some(ref author) = filter.author {
            query.push_str(&format!(" AND author = '{}'", author.replace('\'', "''")));
        }
        query.push_str(&format!(
            " ORDER BY name LIMIT {} OFFSET {}",
            filter.page.page_size,
            (filter.page.page - 1) * filter.page.page_size
        ));

        let rows = sqlx::query_as::<_, SkillSummaryRow>(&query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| SkillError::ConfigValidation(e.to_string()))?;

        rows.into_iter().map(|r| r.into_summary()).collect()
    }

    async fn search(&self, query: &str) -> Result<Vec<SkillSummary>, SkillError> {
        let pattern = format!("%{}%", query);
        let rows = sqlx::query_as::<_, SkillSummaryRow>(
            "SELECT id, name, version, description, author, enabled, tools_json
             FROM skills WHERE name LIKE ? OR description LIKE ? OR author LIKE ?",
        )
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| SkillError::ConfigValidation(e.to_string()))?;

        rows.into_iter().map(|r| r.into_summary()).collect()
    }

    async fn enable(&self, id: &str, enabled: bool) -> Result<(), SkillError> {
        let rows = sqlx::query("UPDATE skills SET enabled = ?, updated_at = ? WHERE id = ?")
            .bind(enabled as i32)
            .bind(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64,
            )
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| SkillError::ConfigValidation(e.to_string()))?
            .rows_affected();
        if rows == 0 {
            return Err(SkillError::NotFound(id.to_string()));
        }
        Ok(())
    }

    async fn count(&self) -> Result<usize, SkillError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM skills")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| SkillError::ConfigValidation(e.to_string()))?;
        Ok(count as usize)
    }
}

// -- Row types for database mapping --

#[cfg(feature = "sqlite-registry")]
#[derive(Debug, sqlx::FromRow)]
struct SkillRow {
    id: String,
    name: String,
    version: String,
    description: String,
    author: String,
    prompt: String,
    tools_json: String,
    permissions_json: String,
    config_schema_json: Option<String>,
    default_config_json: Option<String>,
    enabled: i32,
    created_at: i64,
    updated_at: i64,
    min_agent_version: Option<String>,
}

#[cfg(feature = "sqlite-registry")]
impl SkillRow {
    fn into_skill(self) -> Result<Skill, SkillError> {
        Ok(Skill {
            id: self.id,
            name: self.name,
            version: self.version,
            description: self.description,
            author: self.author,
            prompt: self.prompt,
            tools: serde_json::from_str(&self.tools_json)
                .map_err(|e| SkillError::Yaml(e.to_string()))?,
            permissions: serde_json::from_str(&self.permissions_json)
                .map_err(|e| SkillError::Yaml(e.to_string()))?,
            config_schema: self
                .config_schema_json
                .map(|j| serde_json::from_str(&j))
                .transpose()
                .map_err(|e| SkillError::Yaml(e.to_string()))?,
            default_config: self
                .default_config_json
                .map(|j| serde_json::from_str(&j))
                .transpose()
                .map_err(|e| SkillError::Yaml(e.to_string()))?,
            enabled: self.enabled != 0,
            created_at: self.created_at as u64,
            updated_at: self.updated_at as u64,
            min_agent_version: self.min_agent_version,
        })
    }
}

#[cfg(feature = "sqlite-registry")]
#[derive(Debug, sqlx::FromRow)]
struct SkillSummaryRow {
    id: String,
    name: String,
    version: String,
    description: String,
    author: String,
    enabled: i32,
    tools_json: String,
}

#[cfg(feature = "sqlite-registry")]
impl SkillSummaryRow {
    fn into_summary(self) -> Result<SkillSummary, SkillError> {
        let tools: Vec<serde_json::Value> =
            serde_json::from_str(&self.tools_json).unwrap_or_default();
        Ok(SkillSummary {
            id: self.id,
            name: self.name,
            version: self.version,
            description: self.description,
            author: self.author,
            enabled: self.enabled != 0,
            tool_count: tools.len(),
        })
    }
}
