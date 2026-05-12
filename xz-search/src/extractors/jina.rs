use async_trait::async_trait;
use std::time::Instant;

use crate::error::SearchError;
use crate::traits::{ContentExtractor, ExtractorInfo};
use crate::types::ExtractedContent;

/// Jina Reader API — 内容提取
///
/// 从任意 URL 提取干净的 markdown 文本。
#[derive(Debug)]
pub struct JinaExtractor {
    base_url: String,
    client: reqwest::Client,
    info: ExtractorInfo,
}

impl JinaExtractor {
    pub fn new() -> Self {
        Self {
            base_url: "https://r.jina.ai".into(),
            client: reqwest::Client::new(),
            info: ExtractorInfo {
                name: "jina".into(),
                display_name: "Jina Reader".into(),
                max_url_length: 2048,
                supports_batch: false,
                max_batch_size: 1,
            },
        }
    }
}

impl Default for JinaExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ContentExtractor for JinaExtractor {
    async fn extract(&self, url: &str) -> Result<ExtractedContent, SearchError> {
        let start = Instant::now();

        let reader_url = format!("{}/{}", self.base_url, url);

        let response = self
            .client
            .get(&reader_url)
            .header("Accept", "text/markdown")
            .send()
            .await
            .map_err(|e| SearchError::Extraction {
                url: url.to_string(),
                message: e.to_string(),
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(SearchError::Extraction {
                url: url.to_string(),
                message: format!("HTTP {status}: {body}"),
            });
        }

        let content = response.text().await.map_err(|e| SearchError::Extraction {
            url: url.to_string(),
            message: e.to_string(),
        })?;

        let content_length = content.len();
        let excerpt = content.chars().take(500).collect();

        Ok(ExtractedContent {
            url: url.to_string(),
            title: String::new(),
            content,
            content_length,
            excerpt,
            author: None,
            published_at: None,
            extractor: "jina".into(),
            latency_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn extract_batch(
        &self,
        urls: &[&str],
        concurrency: usize,
    ) -> Result<Vec<ExtractedContent>, SearchError> {
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(concurrency));
        let mut handles = Vec::new();

        for url in urls {
            let url = url.to_string();
            let permit = semaphore.clone().acquire_owned().await.map_err(|_| {
                SearchError::Extraction {
                    url: url.clone(),
                    message: "semaphore closed".into(),
                }
            })?;

            let extractor_url = format!("{}/{}", self.base_url, url);
            let client = self.client.clone();

            handles.push(tokio::spawn(async move {
                let _permit = permit;
                let start = Instant::now();

                let response = client
                    .get(&extractor_url)
                    .header("Accept", "text/markdown")
                    .send()
                    .await?;

                let content = response.text().await?;
                let content_length = content.len();
                let excerpt = content.chars().take(500).collect();

                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(ExtractedContent {
                    url,
                    title: String::new(),
                    content,
                    content_length,
                    excerpt,
                    author: None,
                    published_at: None,
                    extractor: "jina".into(),
                    latency_ms: start.elapsed().as_millis() as u64,
                })
            }));
        }

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(Ok(content)) => results.push(content),
                Ok(Err(e)) => {
                    return Err(SearchError::Extraction {
                        url: "batch".into(),
                        message: e.to_string(),
                    });
                }
                Err(e) => {
                    return Err(SearchError::Extraction {
                        url: "batch".into(),
                        message: e.to_string(),
                    });
                }
            }
        }

        Ok(results)
    }

    fn extractor_info(&self) -> &ExtractorInfo {
        &self.info
    }
}
