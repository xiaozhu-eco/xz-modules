use std::collections::HashMap;
use xz_embed::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = InMemoryVectorStore::new(4);
    store.initialize().await?;

    let mut mock = MockEmbedder::new(4, 32);
    mock.set_output(vec![
        vec![1.0, 0.0, 0.0, 0.0],
        vec![0.0, 1.0, 0.0, 0.0],
        vec![0.0, 0.0, 1.0, 0.0],
    ]);

    let entries = vec![
        make_entry("a", mock.embed(&["rust async"]).await?.remove(0), "rust", "active"),
        make_entry("b", mock.embed(&["python async"]).await?.remove(0), "python", "archived"),
        make_entry("c", mock.embed(&["go async"]).await?.remove(0), "go", "active"),
    ];
    store.insert_batch(entries).await?;

    let filter = MetadataFilter::and([
        MetadataFilter::in_values("lang", &["rust", "go"]),
        MetadataFilter::ne("status", "archived"),
    ]);

    let query = mock.embed(&["async programming"]).await?;
    let results = store.search_with_filter(&query[0], &filter, 10).await?;

    println!("Filtered results:");
    for r in &results {
        println!("  [{:.4}] {} — lang={}", r.score, r.id, r.metadata.get("lang").unwrap_or(&"?".into()));
    }

    Ok(())
}

fn make_entry(id: &str, vector: Vec<f32>, lang: &str, status: &str) -> VectorEntry {
    VectorEntry {
        id: id.to_string(),
        vector,
        content: Some(format!("{lang} async programming")),
        metadata: HashMap::from([
            ("lang".into(), lang.to_string()),
            ("status".into(), status.to_string()),
        ]),
        created_at: 1000,
        expires_at: None,
        channel: Some("test".into()),
    }
}
