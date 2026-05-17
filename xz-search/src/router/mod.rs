pub mod dedup;
pub mod merge;

pub use dedup::deduplicate_by_url;
pub use merge::merge_and_sort;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::stream::{FuturesUnordered, StreamExt};

use crate::cache::memory::MemorySearchCache;
use crate::error::SearchError;
use crate::traits::{ContentExtractor, SearchCache, SearchEngine};
use crate::types::{SearchConfig, SearchItem, SearchOptions, SearchResult};

/// 结果去重策略
#[derive(Debug, Clone)]
pub enum DedupStrategy {
    /// 按 URL 精确去重
    UrlExact,
    /// 按 URL 去重 + 近重复检测
    UrlExactWithNearDup { threshold: f32 },
}

/// 搜索路由器 — 多引擎聚合 + 结果融合 + 缓存
pub struct SearchRouter {
    /// 搜索引擎注册表
    engines: Vec<(String, Box<dyn SearchEngine>)>,
    /// 内容提取器
    extractor: Option<Box<dyn ContentExtractor>>,
    /// 缓存
    cache: Option<Box<dyn SearchCache>>,
    /// 并行搜索的超时
    search_timeout: Duration,
    /// 结果去重策略
    dedup_strategy: DedupStrategy,
    /// 缓存 TTL
    cache_ttl: Duration,
    /// 是否交叉来源
    interleave_sources: bool,
}

impl std::fmt::Debug for SearchRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let engine_names: Vec<&str> = self.engines.iter().map(|(n, _)| n.as_str()).collect();
        f.debug_struct("SearchRouter")
            .field("engines", &engine_names)
            .field("search_timeout", &self.search_timeout)
            .finish()
    }
}

impl SearchRouter {
    pub fn new() -> Self {
        Self {
            engines: Vec::new(),
            extractor: None,
            cache: Some(Box::new(MemorySearchCache::new(1000))),
            search_timeout: Duration::from_secs(10),
            dedup_strategy: DedupStrategy::UrlExact,
            cache_ttl: Duration::from_secs(3600),
            interleave_sources: true,
        }
    }

    /// 注册搜索引擎
    pub fn register_engine(&mut self, engine: Box<dyn SearchEngine>) {
        let name = engine.engine_info().name.clone();
        self.engines.push((name, engine));
    }

    /// 设置内容提取器
    pub fn with_extractor(mut self, extractor: Box<dyn ContentExtractor>) -> Self {
        self.extractor = Some(extractor);
        self
    }

    /// 设置缓存
    pub fn with_cache(mut self, cache: Box<dyn SearchCache>) -> Self {
        self.cache = Some(cache);
        self
    }

    /// 设置搜索超时
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.search_timeout = timeout;
        self
    }

    /// 设置去重策略
    pub fn with_dedup(mut self, strategy: DedupStrategy) -> Self {
        self.dedup_strategy = strategy;
        self
    }

    fn select_engines(&self, names: &[String]) -> Vec<&(String, Box<dyn SearchEngine>)> {
        if names.is_empty() {
            self.engines.iter().collect()
        } else {
            self.engines
                .iter()
                .filter(|(n, _)| names.iter().any(|name| name == n || name == "*"))
                .collect()
        }
    }

    fn make_cache_key(&self, queries: &[String], config: &SearchConfig) -> String {
        let query_hash = queries.join("|");
        format!(
            "search:{}:{}:{}",
            query_hash,
            config.max_results,
            config.sources.join(",")
        )
    }

    /// 核心聚合搜索逻辑
    pub async fn aggregated_search(
        &self,
        query: &str,
        config: &SearchConfig,
        options: &SearchOptions,
    ) -> Result<SearchResult, SearchError> {
        let start = Instant::now();

        // 1. 使用原始查询（查询重写通过可选 feature 启用）
        let queries = vec![query.to_string()];

        // 2. 检查缓存
        if config.enable_cache {
            if let Some(cache) = &self.cache {
                let cache_key = self.make_cache_key(&queries, config);
                if let Some(cached) = cache.get(&cache_key).await {
                    return Ok(cached);
                }
            }
        }

        // 3. 验证引擎选择
        let selected = self.select_engines(&config.engines);
        if selected.is_empty() {
            return Err(SearchError::AllEnginesFailed);
        }

        // 并发查询各引擎
        let mut futures = FuturesUnordered::new();
        for (name, engine) in self.engines.iter() {
            if !config.engines.is_empty() && !config.engines.contains(name) {
                continue;
            }

            let name = name.clone();
            let engine: &dyn SearchEngine = engine.as_ref();
            let query_owned = query.to_string();
            let config_owned = config.clone();
            let options_owned = options.clone();
            let timeout = self.search_timeout;

            futures.push(async move {
                let result = tokio::time::timeout(
                    timeout,
                    engine.search(&query_owned, &config_owned, &options_owned),
                )
                .await;
                (name, result)
            });
        }

        let mut all_items: Vec<SearchItem> = Vec::new();
        let mut engines_used = Vec::new();
        let mut errors = Vec::new();

        while let Some((name, result)) = futures.next().await {
            match result {
                Ok(Ok(result)) => {
                    engines_used.push(name);
                    all_items.extend(result.items);
                }
                Ok(Err(e)) => {
                    errors.push(e);
                }
                Err(_) => {
                    errors.push(SearchError::Timeout(self.search_timeout.as_millis() as u64));
                }
            }
        }

        if all_items.is_empty() && !errors.is_empty() {
            return Err(errors.remove(0));
        }
        if all_items.is_empty() {
            return Err(SearchError::AllEnginesFailed);
        }

        // 5. 去重
        let deduped = deduplicate_by_url(all_items, &self.dedup_strategy);

        // 6. 融合排序
        let merged = merge_and_sort(deduped, config, self.interleave_sources);

        // 7. 自动提取内容
        let mut final_items = merged.items;
        if config.auto_extract {
            if let Some(extractor) = &self.extractor {
                for item in &mut final_items {
                    if item.extracted_content.is_none() {
                        match extractor.extract(&item.url).await {
                            Ok(content) => {
                                item.extracted_content = Some(content);
                            }
                            Err(_) => {} // 提取失败不阻塞
                        }
                    }
                }
            }
        }

        let result = SearchResult {
            query: query.to_string(),
            items: final_items,
            total_results: merged.total_results,
            latency_ms: start.elapsed().as_millis() as u64,
            cached: false,
            engines_used,
            rewritten_query: None,
        };

        // 8. 缓存结果
        if config.enable_cache {
            if let Some(cache) = &self.cache {
                let cache_key = self.make_cache_key(&queries, config);
                cache.set(&cache_key, &result, self.cache_ttl).await;
            }
        }

        Ok(result)
    }
}

impl Default for SearchRouter {
    fn default() -> Self {
        Self::new()
    }
}
