use std::sync::Arc;

use crate::error::SearchError;
use crate::router::SearchRouter;
use crate::types::{SearchConfig, SearchOptions, SearchResult};

/// 批量搜索接口 — 离线批量处理 N 个查询
///
/// 使用场景：批量验证、数据集构建、离线 RAG 上下文收集
pub async fn batch_search(
    router: &SearchRouter,
    queries: &[String],
    config: &SearchConfig,
    concurrency: usize,
) -> Vec<Result<SearchResult, SearchError>> {
    use futures::future;

    let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));

    let handles: Vec<_> = queries
        .iter()
        .map(|query| {
            let query = query.clone();
            let config = config.clone();
            let sem = semaphore.clone();
            // 注意：因为 router 是借用的，我们不能在 spawn 中直接使用它
            // 实际上这需要 Arc<SearchRouter>，这里保留接口定义
            tokio::spawn(async move {
                let _permit = sem.acquire_owned().await.unwrap();
                // 在真正的 Arc 版本中调用 router.aggregated_search
                Err::<SearchResult, SearchError>(SearchError::Config(
                    "batch_search requires Arc<SearchRouter>".into(),
                ))
            })
        })
        .collect();

    let mut results = Vec::new();
    for handle in handles {
        match handle.await {
            Ok(Ok(r)) => results.push(Ok(r)),
            Ok(Err(e)) => results.push(Err(e)),
            Err(e) => results.push(Err(SearchError::Config(e.to_string()))),
        }
    }
    results
}

/// Arc-wrapped batch search（推荐使用）
pub async fn batch_search_with_arc(
    router: Arc<SearchRouter>,
    queries: &[String],
    config: &SearchConfig,
    concurrency: usize,
) -> Vec<Result<SearchResult, SearchError>> {
    use futures::future;

    let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));

    let handles: Vec<_> = queries
        .iter()
        .map(|query| {
            let query = query.clone();
            let config = config.clone();
            let router = router.clone();
            let sem = semaphore.clone();

            tokio::spawn(async move {
                let _permit = sem.acquire_owned().await.unwrap();
                router
                    .aggregated_search(&query, &config, &SearchOptions::default())
                    .await
            })
        })
        .collect();

    let mut results = Vec::new();
    for handle in handles {
        match handle.await {
            Ok(r) => results.push(r),
            Err(e) => results.push(Err(SearchError::Config(e.to_string()))),
        }
    }
    results
}
