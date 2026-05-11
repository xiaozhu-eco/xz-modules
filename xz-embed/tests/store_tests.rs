use std::collections::HashMap;
use xz_embed::*;

fn make_entry(id: &str, vector: Vec<f32>, lang: &str, status: &str) -> VectorEntry {
    VectorEntry {
        id: id.to_string(),
        vector,
        content: Some(format!("content-{id}")),
        metadata: HashMap::from([
            ("lang".into(), lang.to_string()),
            ("status".into(), status.to_string()),
        ]),
        created_at: 1000,
        expires_at: None,
        channel: Some("test".into()),
    }
}

#[tokio::test]
async fn test_memory_store_insert_and_search() {
    let store = InMemoryVectorStore::new(4);
    store.initialize().await.unwrap();

    let entries = vec![
        VectorEntry {
            id: "doc1".into(),
            vector: vec![1.0, 0.0, 0.0, 0.0],
            content: Some("rust programming".into()),
            metadata: HashMap::from([("lang".into(), "rust".into())]),
            created_at: 1000,
            expires_at: None,
            channel: Some("test".into()),
        },
        VectorEntry {
            id: "doc2".into(),
            vector: vec![0.0, 1.0, 0.0, 0.0],
            content: Some("python data science".into()),
            metadata: HashMap::from([("lang".into(), "python".into())]),
            created_at: 1000,
            expires_at: None,
            channel: Some("test".into()),
        },
    ];
    store.insert_batch(entries).await.unwrap();

    let results = store.search(&[0.9, 0.1, 0.0, 0.0], 2).await.unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].id, "doc1");
    assert!(results[0].score > results[1].score);
}

#[tokio::test]
async fn test_metadata_filter() {
    let store = InMemoryVectorStore::new(2);
    store.initialize().await.unwrap();

    let entries = vec![
        make_entry("a", vec![1.0, 0.0], "rust", "active"),
        make_entry("b", vec![0.8, 0.2], "rust", "archived"),
        make_entry("c", vec![0.1, 0.9], "python", "active"),
    ];
    store.insert_batch(entries).await.unwrap();

    let filter = MetadataFilter::and([
        MetadataFilter::eq("lang", "rust"),
        MetadataFilter::ne("status", "archived"),
    ]);

    let results = store
        .search_with_filter(&[1.0, 0.0], &filter, 5)
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "a");
}

#[tokio::test]
async fn test_delete_and_count() {
    let store = InMemoryVectorStore::new(2);
    store.initialize().await.unwrap();

    store
        .insert_batch(vec![
            make_entry("a", vec![1.0, 0.0], "rust", "active"),
            make_entry("b", vec![0.8, 0.2], "rust", "archived"),
        ])
        .await
        .unwrap();

    assert_eq!(store.count().await.unwrap(), 2);

    let deleted = store.delete(&["a".into()]).await.unwrap();
    assert_eq!(deleted, 1);
    assert_eq!(store.count().await.unwrap(), 1);
}

#[tokio::test]
async fn test_dimension_mismatch() {
    let store = InMemoryVectorStore::new(4);
    store.initialize().await.unwrap();

    let result = store
        .insert(VectorEntry {
            id: "bad".into(),
            vector: vec![1.0, 0.0], // only 2 dims
            content: None,
            metadata: HashMap::new(),
            created_at: 0,
            expires_at: None,
            channel: None,
        })
        .await;

    assert!(matches!(
        result.unwrap_err(),
        StoreError::DimensionMismatch { .. }
    ));
}
