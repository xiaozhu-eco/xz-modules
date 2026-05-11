use serde::{Deserialize, Serialize};

/// Configuration for the memory system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub storage: StorageConfig,
    pub short_term: ShortTermConfig,
    #[cfg(feature = "summary")]
    pub summary: SummaryConfig,
    pub compaction: CompactionConfig,
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
            path: "./data/memory.db".into(),
            pool_size: 5,
        }
    }
}

/// Short-term memory configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortTermConfig {
    pub max_messages_per_session: usize,
    pub message_retention_days: u32,
}

impl Default for ShortTermConfig {
    fn default() -> Self {
        Self {
            max_messages_per_session: 100,
            message_retention_days: 30,
        }
    }
}

/// Summary generation configuration.
#[cfg(feature = "summary")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryConfig {
    pub enabled: bool,
    pub model: String,
    pub trigger_at_message_count: usize,
    pub max_summary_length: usize,
}

#[cfg(feature = "summary")]
impl Default for SummaryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            model: "gpt-4o-mini".into(),
            trigger_at_message_count: 20,
            max_summary_length: 500,
        }
    }
}

/// Compaction configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionConfig {
    pub auto_compact_interval_hrs: u32,
    pub default_strategy: String,
    pub low_confidence_threshold: f32,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            auto_compact_interval_hrs: 24,
            default_strategy: "merge_similar".into(),
            low_confidence_threshold: 0.3,
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

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            storage: StorageConfig::default(),
            short_term: ShortTermConfig::default(),
            #[cfg(feature = "summary")]
            summary: SummaryConfig::default(),
            compaction: CompactionConfig::default(),
            fts: FtsConfig::default(),
        }
    }
}
