use xz_search::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut router = SearchRouter::new();

    // 注册 Mock 引擎用于演示
    let mut mock = MockSearchEngine::new("demo");
    router.register_engine(Box::new(mock));

    let result = router
        .aggregated_search(
            "Rust 2024 edition new features",
            &SearchConfig {
                max_results: 5,
                engines: vec!["demo".into()],
                ..Default::default()
            },
            &SearchOptions::default(),
        )
        .await?;

    println!("Search results ({}ms):", result.latency_ms);
    for (i, item) in result.items.iter().enumerate() {
        println!(
            "  {}. [{}] {} - {}",
            i + 1, item.source, item.title, item.url
        );
        println!("     {}", item.snippet);
    }
    println!("Total: {} results, cached: {}", result.total_results, result.cached);

    Ok(())
}
