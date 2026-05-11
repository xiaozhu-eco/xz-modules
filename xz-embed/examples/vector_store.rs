use std::collections::HashMap;
use xz_embed::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = InMemoryVectorStore::new(4);
    store.initialize().await?;

    let mut mock = MockEmbedder::new(4, 32);
    mock.set_output(vec![
        vec![0.1, 0.2, 0.3, 0.4],
        vec![0.5, 0.6, 0.7, 0.8],
    ]);

    let texts = vec!["Rust programming", "Python data science"];
    let text_refs: Vec<&str> = texts.iter().copied().collect();
    let vectors = mock.embed(&text_refs).await?;

    let entries: Vec<VectorEntry> = texts
        .iter()
        .zip(vectors.iter())
        .enumerate()
        .map(|(i, (text, vec))| VectorEntry {
            id: uuid::Uuid::new_v4().to_string(),
            vector: vec.clone(),
            metadata: HashMap::from([
                ("source".into(), "docs".into()),
                ("lang".into(), if i == 0 { "rust" } else { "python" }.into()),
            ]),
            content: Some(text.to_string()),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            expires_at: None,
            channel: Some("semantic".into()),
        })
        .collect();

    store.insert_batch(entries).await?;

    let query_vec = mock.embed(&["systems programming"]).await?;
    let results = store.search(&query_vec[0], 5).await?;

    for r in &results {
        println!("[{:.4}] {} — {:?}", r.score, r.id, r.content);
    }

    Ok(())
}
