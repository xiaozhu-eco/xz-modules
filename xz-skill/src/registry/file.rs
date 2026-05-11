use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use async_trait::async_trait;

use crate::error::SkillError;
use crate::traits::SkillRegistry;
use crate::types::filter::SkillFilter;
use crate::types::output::SkillSummary;
use crate::types::skill::{Skill, UpsertResult};

/// File-based skill registry.
///
/// Expected layout: `{base_dir}/{skill_id}/skill.yaml` plus optional `*.wasm` files.
#[derive(Debug)]
pub struct FileSkillRegistry {
    base_dir: PathBuf,
    skills: RwLock<HashMap<String, Skill>>,
}

impl FileSkillRegistry {
    pub async fn new(base_dir: &Path) -> Result<Self, SkillError> {
        let registry = Self {
            base_dir: base_dir.to_path_buf(),
            skills: RwLock::new(HashMap::new()),
        };
        registry.load_all().await?;
        Ok(registry)
    }

    /// Re-scan the directory and reload all skills.
    pub async fn reload(&self) -> Result<usize, SkillError> {
        self.skills.write().unwrap().clear();
        self.load_all().await
    }

    /// Load all skills from the directory.
    pub async fn load_all(&self) -> Result<usize, SkillError> {
        let mut count = 0;
        if !self.base_dir.exists() {
            return Ok(0);
        }
        if !self.base_dir.is_dir() {
            return Ok(0);
        }

        let entries = std::fs::read_dir(&self.base_dir)?;
        for entry in entries {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let yaml_path = entry.path().join("skill.yaml");
                if yaml_path.exists() {
                    match self.load_skill_from_yaml(&yaml_path) {
                        Ok(mut skill) => {
                            // Load WASM modules if any
                            for tool in &mut skill.tools {
                                if let crate::types::skill::ToolType::Wasm {
                                    ref mut module,
                                    ref module_path,
                                    ..
                                } = tool.tool_type
                                {
                                    if let Some(mpath) = module_path {
                                        let wasm_path = entry.path().join(mpath);
                                        if wasm_path.exists() {
                                            *module = std::fs::read(&wasm_path)?;
                                        }
                                    }
                                }
                            }
                            self.skills.write().unwrap().insert(skill.id.clone(), skill);
                            count += 1;
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to load skill from {}: {}",
                                yaml_path.display(),
                                e
                            );
                        }
                    }
                }
            }
        }

        Ok(count)
    }

    fn load_skill_from_yaml(&self, path: &Path) -> Result<Skill, SkillError> {
        let content = std::fs::read_to_string(path)?;
        serde_yaml::from_str(&content).map_err(|e| SkillError::Yaml(e.to_string()))
    }

    #[cfg(feature = "hot-reload")]
    pub async fn watch(&self) -> Result<(), SkillError> {
        use notify::{Event, EventKind, RecursiveMode, Watcher};
        use std::time::Duration;

        let (tx, mut rx) = tokio::sync::mpsc::channel(32);
        let base_dir = self.base_dir.clone();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                    let _ = tx.try_send(());
                }
            }
        })
        .map_err(|e| SkillError::ConfigValidation(e.to_string()))?;

        watcher
            .watch(&base_dir, RecursiveMode::Recursive)
            .map_err(|e| SkillError::ConfigValidation(e.to_string()))?;

        loop {
            if rx.recv().await.is_some() {
                // Debounce: wait a bit for writes to finish
                tokio::time::sleep(Duration::from_millis(500)).await;
                if let Err(e) = self.reload().await {
                    tracing::warn!("Hot-reload error: {}", e);
                }
            }
        }
    }
}

#[async_trait]
impl SkillRegistry for FileSkillRegistry {
    async fn register(&self, skill: Skill) -> Result<UpsertResult, SkillError> {
        let mut skills = self.skills.write().unwrap();
        if let Some(existing) = skills.get(&skill.id) {
            let mut changed_fields = Vec::new();
            if existing.name != skill.name {
                changed_fields.push("name".into());
            }
            if existing.version != skill.version {
                changed_fields.push("version".into());
            }
            if existing.description != skill.description {
                changed_fields.push("description".into());
            }
            if existing.prompt != skill.prompt {
                changed_fields.push("prompt".into());
            }
            if changed_fields.is_empty() {
                return Ok(UpsertResult::Unchanged);
            }
            skills.insert(skill.id.clone(), skill);
            Ok(UpsertResult::Updated { changed_fields })
        } else {
            skills.insert(skill.id.clone(), skill);
            Ok(UpsertResult::Created)
        }
    }

    async fn unregister(&self, id: &str) -> Result<(), SkillError> {
        self.skills
            .write()
            .unwrap()
            .remove(id)
            .map(|_| ())
            .ok_or_else(|| SkillError::NotFound(id.to_string()))
    }

    async fn get(&self, id: &str) -> Result<Option<Skill>, SkillError> {
        Ok(self.skills.read().unwrap().get(id).cloned())
    }

    async fn list(&self, filter: &SkillFilter) -> Result<Vec<SkillSummary>, SkillError> {
        let skills = self.skills.read().unwrap();
        let mut summaries: Vec<SkillSummary> = skills
            .values()
            .filter(|s| {
                if filter.enabled_only && !s.enabled {
                    return false;
                }
                if let Some(ref author) = filter.author {
                    if s.author != *author {
                        return false;
                    }
                }
                true
            })
            .map(|s| SkillSummary {
                id: s.id.clone(),
                name: s.name.clone(),
                version: s.version.clone(),
                description: s.description.clone(),
                author: s.author.clone(),
                enabled: s.enabled,
                tool_count: s.tools.len(),
            })
            .collect();

        let offset = (filter.page.page - 1) * filter.page.page_size;
        summaries.sort_by(|a, b| a.name.cmp(&b.name));

        if offset < summaries.len() {
            summaries = summaries
                .into_iter()
                .skip(offset)
                .take(filter.page.page_size)
                .collect();
        } else {
            summaries.clear();
        }

        Ok(summaries)
    }

    async fn search(&self, query: &str) -> Result<Vec<SkillSummary>, SkillError> {
        let lower = query.to_lowercase();
        let skills = self.skills.read().unwrap();
        Ok(skills
            .values()
            .filter(|s| {
                s.name.to_lowercase().contains(&lower)
                    || s.description.to_lowercase().contains(&lower)
                    || s.author.to_lowercase().contains(&lower)
            })
            .map(|s| SkillSummary {
                id: s.id.clone(),
                name: s.name.clone(),
                version: s.version.clone(),
                description: s.description.clone(),
                author: s.author.clone(),
                enabled: s.enabled,
                tool_count: s.tools.len(),
            })
            .collect())
    }

    async fn enable(&self, id: &str, enabled: bool) -> Result<(), SkillError> {
        let mut skills = self.skills.write().unwrap();
        let skill = skills
            .get_mut(id)
            .ok_or_else(|| SkillError::NotFound(id.to_string()))?;
        skill.enabled = enabled;
        skill.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Ok(())
    }

    async fn count(&self) -> Result<usize, SkillError> {
        Ok(self.skills.read().unwrap().len())
    }
}
