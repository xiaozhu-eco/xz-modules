use async_trait::async_trait;
use std::sync::Mutex;
use std::time::Instant;

use crate::error::SearchError;
use crate::traits::{SearchEngine, SearchEngineInfo};
use crate::types::{SearchConfig, SearchItem, SearchOptions, SearchResult};

/// 测试用 Mock 搜索引擎
#[derive(Debug)]
pub struct MockSearchEngine {
    info: SearchEngineInfo,
    mock_results: Mutex<Option<Vec<SearchItem>>>,
    should_error: Mutex<Option<SearchError>>,
    delay: Mutex<Option<std::time::Duration>>,
}

impl MockSearchEngine {
    pub fn new(name: &str) -> Self {
        Self {
            info: SearchEngineInfo {
                name: name.to_string(),
                display_name: format!("Mock {name}"),
                description: "Mock search engine for testing".into(),
                supported_sources: vec!["web".into()],
                max_results: 100,
                supported_regions: vec![],
                supports_time_range: false,
                pricing: None,
            },
            mock_results: Mutex::new(None),
            should_error: Mutex::new(None),
            delay: Mutex::new(None),
        }
    }

    /// 设置返回结果
    pub fn set_results(&mut self, items: Vec<SearchItem>) {
        *self.mock_results.get_mut().unwrap() = Some(items);
    }

    /// 设置错误
    pub fn set_error(&mut self, error: SearchError) {
        *self.should_error.get_mut().unwrap() = Some(error);
    }

    /// 设置模拟延迟（用于测试并发执行）
    pub fn set_delay(&mut self, delay: std::time::Duration) {
        *self.delay.get_mut().unwrap() = Some(delay);
    }
}

#[async_trait]
impl SearchEngine for MockSearchEngine {
    async fn search(
        &self,
        query: &str,
        config: &SearchConfig,
        _options: &SearchOptions,
    ) -> Result<SearchResult, SearchError> {
        let start = Instant::now();

        if let Some(ref err) = *self.should_error.lock().unwrap() {
            return Err(SearchError::Api {
                engine: self.info.name.clone(),
                message: format!("Mock error: {err}"),
            });
        }

        // 模拟延迟
        let delay = *self.delay.lock().unwrap();
        if let Some(delay) = delay {
            tokio::time::sleep(delay).await;
        }

        let items = match self.mock_results.lock().unwrap().take() {
            Some(items) => items,
            None => vec![SearchItem {
                title: format!("Mock result for: {query}"),
                url: format!("https://{}.example.com/mock?q={}", self.info.name, urlencoding(query)),
                snippet: format!("This is a mock result from {} for query: {query}", self.info.name),
                source: self.info.name.clone(),
                published_at: None,
                score: 0.9,
                domain: "example.com".into(),
                detected_language: None,
                extracted_content: None,
            }],
        };

        let total = items.len() as u64;
        let items = items
            .into_iter()
            .skip(config.offset)
            .take(config.max_results)
            .collect();

        Ok(SearchResult {
            query: query.to_string(),
            items,
            total_results: total,
            latency_ms: start.elapsed().as_millis() as u64,
            cached: false,
            engines_used: vec![self.info.name.clone()],
            rewritten_query: None,
        })
    }

    fn engine_info(&self) -> &SearchEngineInfo {
        &self.info
    }
}

fn urlencoding(s: &str) -> String {
    percent_encoding::utf8_percent_encode(s, percent_encoding::NON_ALPHANUMERIC).to_string()
}
