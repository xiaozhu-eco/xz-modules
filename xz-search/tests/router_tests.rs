use std::time::{Duration, Instant};

use xz_search::*;

#[tokio::test]
async fn test_search_router_aggregation() {
    let mut mock1 = MockSearchEngine::new("mock1");

    let mut mock2 = MockSearchEngine::new("mock2");

    let mut router = SearchRouter::new();
    router.register_engine(Box::new(mock1));
    router.register_engine(Box::new(mock2));

    let result = router
        .aggregated_search("test", &SearchConfig::default(), &SearchOptions::default())
        .await
        .unwrap();

    assert_eq!(result.items.len(), 2);
    assert_eq!(result.engines_used.len(), 2);
}

#[tokio::test]
async fn test_router_all_engines_failed() {
    let router = SearchRouter::new();

    let result = router
        .aggregated_search("test", &SearchConfig::default(), &SearchOptions::default())
        .await;

    assert!(matches!(result.unwrap_err(), SearchError::AllEnginesFailed));
}

#[tokio::test]
async fn test_engine_error() {
    let mut mock = MockSearchEngine::new("mock");
    mock.set_error(SearchError::Api {
        engine: "mock".into(),
        message: "test error".into(),
    });

    let result = mock
        .search("test", &SearchConfig::default(), &SearchOptions::default())
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn urlencoding_correct() {
    let mock = MockSearchEngine::new("mock");
    let result = mock
        .search("hello world", &SearchConfig::default(), &SearchOptions::default())
        .await
        .unwrap();

    assert_eq!(result.items.len(), 1);
    // percent_encoding::utf8_percent_encode encodes space as %20
    let url = &result.items[0].url;
    assert!(url.contains("hello%20world"), "URL should contain percent-encoded space: {}", url);
}

#[tokio::test]
async fn router_parallel_engines() {
    let delay = Duration::from_millis(200);

    let mut mock1 = MockSearchEngine::new("mock1");
    mock1.set_delay(delay);
    let mut mock2 = MockSearchEngine::new("mock2");
    mock2.set_delay(delay);

    let mut router = SearchRouter::new();
    router.register_engine(Box::new(mock1));
    router.register_engine(Box::new(mock2));

    let start = Instant::now();
    let result = router
        .aggregated_search("test", &SearchConfig::default(), &SearchOptions::default())
        .await
        .unwrap();

    let elapsed = start.elapsed();
    assert_eq!(result.engines_used.len(), 2);

    // 并发执行：总耗时应接近 max(delay) 而非 sum(delay)
    // 容差 50% 以防止 CI 环境波动
    let max_delay = delay;
    let sum_delay = delay * 2;
    assert!(
        elapsed < max_delay + (sum_delay - max_delay) / 2,
        "Engines should run concurrently. Elapsed: {:?}, Max delay: {:?}, Sum delay: {:?}",
        elapsed, max_delay, sum_delay
    );
}
