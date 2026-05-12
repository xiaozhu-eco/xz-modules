use serde::{Deserialize, Serialize};

use crate::types::import::MergeStrategy;
use crate::types::relation::WeightStrategy;

/// Knowledge graph configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KgConfig {
    pub storage: StorageConfig,
    pub merge_strategy: MergeStrategy,
    pub weight_strategy: WeightStrategy,
    pub max_bfs_depth: u32,
    pub max_path_search: u32,
    pub fts: FtsConfig,
}

/// Storage backend configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub backend: String,
    pub path: String,
    pub pool_size: u32,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend: "sqlite".into(),
            path: "./data/knowledge_graph.db".into(),
            pool_size: 5,
        }
    }
}

/// FTS configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FtsConfig {
    pub enabled: bool,
    pub min_query_length: usize,
}

impl Default for FtsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_query_length: 2,
        }
    }
}

impl Default for KgConfig {
    fn default() -> Self {
        Self {
            storage: StorageConfig::default(),
            merge_strategy: MergeStrategy::Replace,
            weight_strategy: WeightStrategy::InverseConfidence,
            max_bfs_depth: 5,
            max_path_search: 10,
            fts: FtsConfig::default(),
        }
    }
}
