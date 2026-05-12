use xz_search::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut router = SearchRouter::new();

    // 注册多个 Mock 引擎
    let mut mock1 = MockSearchEngine::new("tavily-mock");
    let mut mock2 = MockSearchEngine::new("serpapi-mock");

    mock1.set_results(vec![SearchItem {
        title: "Rust Official Blog".into(),
        url: "https://blog.rust-lang.org/2024-edition".into(),
        snippet: "The Rust 2024 edition brings new features...".into(),
        source: "tavily-mock".into(),
        published_at: None,
        score: 0.95,
        domain: "blog.rust-lang.org".into(),
        detected_language: None,
        extracted_content: None,
    }]);

    mock2.set_results(vec![SearchItem {
        title: "Rust 2024 Migration Guide".into(),
        url: "https://doc.rust-lang.org/edition-2024".into(),
        snippet: "How to migrate your project to Rust 2024 edition...".into(),
        source: "serpapi-mock".into(),
        published_at: None,
        score: 0.85,
        domain: "doc.rust-lang.org".into(),
        detected_language: None,
        extracted_content: None,
    }]);

    router.register_engine(Box::new(mock1));
    router.register_engine(Box::new(mock2));

    let result = router
        .aggregated_search(
            "Rust 2024 edition",
            &SearchConfig {
                max_results: 5,
                ..Default::default()
            },
            &SearchOptions::default(),
        )
        .await?;

    println!("Multi-engine search results:");
    println!("  Engines used: {:?}", result.engines_used);
    for item in &result.items {
        println!("  [{:.2}] {} ({})", item.score, item.title, item.source);
    }

    Ok(())
}
