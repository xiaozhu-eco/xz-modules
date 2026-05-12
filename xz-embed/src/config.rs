use serde::{Deserialize, Serialize};

use crate::error::EmbedError;
use crate::traits::TruncationStrategy;

/// 模型配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub provider: String,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub dimensions: usize,
    pub max_batch_size: usize,
    pub max_input_tokens: usize,
    pub input_per_million: f64,
    /// 本地模型路径
    pub model_path: Option<String>,
    pub tokenizer_path: Option<String>,
    pub base_url: Option<String>,
}

/// 存储配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub backend: String,
    pub path: String,
    pub max_capacity_bytes: Option<u64>,
    pub table_name: String,
}

/// 索引构建模式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexBuildMode {
    Incremental,
    Manual,
    Deferred { interval_ms: u64 },
}

/// 索引重建触发器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RebuildTrigger {
    Count(usize),
    IntervalSecs(u64),
    ManualOnly,
}

/// 索引构建配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexBuildConfig {
    pub mode: IndexBuildMode,
    pub rebuild_trigger: RebuildTrigger,
}

/// 完整嵌入配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedConfig {
    /// 默认模型名称
    pub default_model: String,
    /// 默认向量维度
    pub default_dimensions: usize,
    /// 截断策略
    pub truncation: TruncationStrategy,
    /// 存储配置
    pub storage: StorageConfig,
    /// 索引配置
    pub index: IndexBuildConfig,
    /// 模型列表
    pub models: Vec<ModelConfig>,
}

impl EmbedConfig {
    /// 查找模型配置
    pub fn find_model(&self, name: &str) -> Option<&ModelConfig> {
        self.models.iter().find(|m| {
            m.model.as_deref() == Some(name) || m.provider == name
        })
    }

    /// 查找默认模型配置
    pub fn default_model_config(&self) -> Result<&ModelConfig, EmbedError> {
        self.find_model(&self.default_model)
            .ok_or_else(|| EmbedError::Config(format!("默认模型未找到: {}", self.default_model)))
    }
}

/// 重试配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 10_000,
            backoff_multiplier: 2.0,
        }
    }
}
