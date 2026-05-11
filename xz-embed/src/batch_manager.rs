use std::sync::Arc;
use std::time::Duration;

use futures::future;
use tokio::sync::Semaphore;
use tracing::{debug, warn};

use crate::config::RetryConfig;
use crate::error::EmbedError;
use crate::traits::EmbeddingModel;

/// 并发批量管理器
///
/// 将大批次文本拆分为多个小批次，并发发送 Embedding API 请求。
/// 内置重试、限流、进度回调。
pub struct ConcurrentBatchManager {
    embedder: Arc<dyn EmbeddingModel>,
    /// 每批文本数（不超过 embedder.max_batch_size()）
    batch_size: usize,
    /// 最大并发批次
    max_concurrency: usize,
    /// 重试策略
    retry: RetryConfig,
}

impl ConcurrentBatchManager {
    pub fn new(embedder: Box<dyn EmbeddingModel>, batch_size: usize, max_concurrency: usize) -> Self {
        Self {
            embedder: Arc::from(embedder),
            batch_size,
            max_concurrency,
            retry: RetryConfig::default(),
        }
    }

    pub fn with_retry(mut self, retry: RetryConfig) -> Self {
        self.retry = retry;
        self
    }

    /// 将文本列表拆分为多个子批次
    fn chunk_texts(&self, texts: &[impl AsRef<str>]) -> Vec<Vec<String>> {
        let max_batch = self
            .batch_size
            .min(self.embedder.max_batch_size());
        texts
            .chunks(max_batch)
            .map(|chunk| chunk.iter().map(|t| t.as_ref().to_string()).collect())
            .collect()
    }

    /// 嵌入全部文本，返回顺序与输入一致
    pub async fn embed_all(
        &self,
        texts: &[impl AsRef<str>],
    ) -> Result<Vec<Vec<f32>>, EmbedError> {
        self.embed_all_with_progress(texts, |_, _| {}).await
    }

    /// 带进度回调的嵌入
    pub async fn embed_all_with_progress(
        &self,
        texts: &[impl AsRef<str>],
        on_batch_done: impl Fn(usize, usize),
    ) -> Result<Vec<Vec<f32>>, EmbedError> {
        let batches = self.chunk_texts(texts);
        let total_batches = batches.len();

        if total_batches == 0 {
            return Ok(vec![]);
        }

        let semaphore = Arc::new(Semaphore::new(self.max_concurrency));
        let mut handles = Vec::with_capacity(total_batches);

        for (i, batch) in batches.into_iter().enumerate() {
            let permit = semaphore.clone().acquire_owned().await.map_err(|e| {
                EmbedError::Config(format!("获取信号量失败: {e}"))
            })?;

            let embedder = self.embedder.clone();
            let retry = self.retry.clone();

            handles.push(tokio::spawn(async move {
                let _permit = permit;
                let texts_refs: Vec<&str> = batch.iter().map(|s| s.as_str()).collect();
                let result = retry_with_backoff(|| embedder.embed(&texts_refs), &retry).await;
                (i, result)
            }));
        }

        let mut ordered_results: Vec<Option<Vec<Vec<f32>>>> = vec![None; total_batches];
        let mut errors = Vec::new();

        for handle in handles {
            match handle.await {
                Ok((idx, Ok(vectors))) => {
                    ordered_results[idx] = Some(vectors);
                    on_batch_done(idx + 1, total_batches);
                }
                Ok((idx, Err(e))) => {
                    warn!(target: "xz_embed", batch = idx, error = %e, "batch embedding failed");
                    errors.push(e);
                }
                Err(e) => {
                    errors.push(EmbedError::Config(format!("task join error: {e}")));
                }
            }
        }

        if !errors.is_empty() {
            return Err(errors.remove(0));
        }

        let all_vectors: Vec<Vec<f32>> = ordered_results
            .into_iter()
            .filter_map(|r| r)
            .flatten()
            .collect();

        debug!(
            target: "xz_embed",
            total_texts = texts.len(),
            total_batches,
            total_vectors = all_vectors.len(),
            "embed_all completed"
        );

        Ok(all_vectors)
    }
}

async fn retry_with_backoff<F, Fut, T>(
    f: F,
    config: &RetryConfig,
) -> Result<T, EmbedError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, EmbedError>>,
{
    let mut attempt = 0;
    let mut backoff_ms = config.initial_backoff_ms;

    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) if e.is_retryable() && attempt < config.max_retries => {
                attempt += 1;
                debug!(
                    target: "xz_embed",
                    attempt,
                    backoff_ms,
                    error = %e,
                    "retrying embedding request"
                );
                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                backoff_ms =
                    (backoff_ms as f64 * config.backoff_multiplier).min(config.max_backoff_ms as f64)
                        as u64;
            }
            Err(e) => return Err(e),
        }
    }
}
