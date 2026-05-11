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
