use std::collections::HashMap;
use xz_embed::*;

#[tokio::test]
async fn test_mock_embedder() {
    let mut mock = MockEmbedder::new(384, 32);
    mock.set_output(vec![vec![0.1; 384], vec![0.2; 384]]);

    let vectors = mock.embed(&["hello", "world"]).await.unwrap();
    assert_eq!(vectors.len(), 2);
    assert_eq!(vectors[0].len(), 384);
    assert_eq!(mock.dimensions(), 384);
}

#[tokio::test]
async fn test_batch_embed_with_expectation() {
    let mut mock = MockEmbedder::new(4, 32);
    mock.expect_embed(
        vec!["hello", "world"],
        vec![vec![0.1; 4], vec![0.2; 4]],
    );

    let vectors = mock.embed(&["hello", "world"]).await.unwrap();
    assert_eq!(vectors.len(), 2);
    assert_eq!(vectors[0].len(), 4);
}

#[tokio::test]
async fn test_embed_error_empty_batch() {
    let mock = MockEmbedder::new(4, 32);
    let result = mock.embed(&[]).await;
    assert!(matches!(result.unwrap_err(), EmbedError::EmptyBatch));
}
