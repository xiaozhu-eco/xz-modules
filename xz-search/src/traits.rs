use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::time::Duration;

use crate::error::SearchError;
use crate::types::{ExtractedContent, SearchConfig, SearchOptions, SearchResult};

/// 搜索引擎信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchEngineInfo {
    pub name: String,
    pub display_name: String,
    pub description: String,
    /// 支持的源
    pub supported_sources: Vec<String>,
    /// 支持的最大结果数
    pub max_results: usize,
    /// 支持的地区
    pub supported_regions: Vec<String>,
    /// 是否支持时间范围过滤
    pub supports_time_range: bool,
    /// 定价
    pub pricing: Option<SearchPricing>,
}

/// 搜索引擎定价
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchPricing {
    pub cost_per_search: f64,
}

/// 内容提取器信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractorInfo {
    pub name: String,
    pub display_name: String,
    pub max_url_length: usize,
    pub supports_batch: bool,
    pub max_batch_size: usize,
}

/// 统一的搜索引擎接口
#[async_trait]
pub trait SearchEngine: Send + Sync + Debug {
    /// 执行搜索
    async fn search(
        &self,
        query: &str,
        config: &SearchConfig,
        options: &SearchOptions,
    ) -> Result<SearchResult, SearchError>;

    /// 搜索引擎信息
    fn engine_info(&self) -> &SearchEngineInfo;
}

/// 内容提取器接口
#[async_trait]
pub trait ContentExtractor: Send + Sync + Debug {
    /// 提取单条 URL 内容
    async fn extract(&self, url: &str) -> Result<ExtractedContent, SearchError>;

    /// 批量提取
    async fn extract_batch(
        &self,
        urls: &[&str],
        concurrency: usize,
    ) -> Result<Vec<ExtractedContent>, SearchError>;

    /// 提取器信息
    fn extractor_info(&self) -> &ExtractorInfo;
}

/// 搜索缓存 trait
#[async_trait]
pub trait SearchCache: Send + Sync + Debug {
    /// 获取缓存结果
    async fn get(&self, key: &str) -> Option<SearchResult>;

    /// 存储缓存结果
    async fn set(&self, key: &str, result: &SearchResult, ttl: Duration);

    /// 使缓存失效
    async fn invalidate(&self, key: &str);

    /// 缓存命中统计
    fn stats(&self) -> CacheStats;
}

/// 缓存统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub size_bytes: u64,
    pub entry_count: usize,
}
