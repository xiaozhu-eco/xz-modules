use async_trait::async_trait;
use std::sync::Mutex;

use crate::error::SearchError;
use crate::traits::{ContentExtractor, ExtractorInfo};
use crate::types::ExtractedContent;

/// 测试用 Mock 内容提取器
#[derive(Debug)]
pub struct MockExtractor {
    info: ExtractorInfo,
    mock_content: Mutex<Option<ExtractedContent>>,
}

impl MockExtractor {
    pub fn new() -> Self {
        Self {
            info: ExtractorInfo {
                name: "mock".into(),
                display_name: "Mock Extractor".into(),
                max_url_length: 2048,
                supports_batch: true,
                max_batch_size: 100,
            },
            mock_content: Mutex::new(None),
        }
    }

    pub fn set_content(&mut self, content: ExtractedContent) {
        *self.mock_content.get_mut().unwrap() = Some(content);
    }
}

impl Default for MockExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ContentExtractor for MockExtractor {
    async fn extract(&self, url: &str) -> Result<ExtractedContent, SearchError> {
        if let Some(ref content) = *self.mock_content.lock().unwrap() {
            return Ok(content.clone());
        }

        Ok(ExtractedContent {
            url: url.to_string(),
            title: "Mock Page".into(),
            content: format!("Mock content for {url}"),
            content_length: 100,
            excerpt: "Mock excerpt...".into(),
            author: None,
            published_at: None,
            extractor: "mock".into(),
            latency_ms: 1,
        })
    }

    async fn extract_batch(
        &self,
        urls: &[&str],
        _concurrency: usize,
    ) -> Result<Vec<ExtractedContent>, SearchError> {
        let mut results = Vec::new();
        for url in urls {
            results.push(self.extract(url).await?);
        }
        Ok(results)
    }

    fn extractor_info(&self) -> &ExtractorInfo {
        &self.info
    }
}
