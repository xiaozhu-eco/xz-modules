use xz_rag::{
    indexing::DocumentIndexer,
    FixedSizeChunker, RecursiveCharacterChunker, SemanticChunker,
    ChunkStrategy,
    ChunkMetadata, IndexDocument,
};
use std::sync::Arc;

#[test]
fn test_fixed_size_chunker() {
    let chunker = FixedSizeChunker::new(10, 2);
    let text = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let chunks = chunker.chunk(text);
    assert!(chunks.len() > 1);
    for c in &chunks {
        assert!(c.len() <= 10 + 2);
    }
}

#[test]
fn test_recursive_chunker() {
    let chunker = RecursiveCharacterChunker::new(
        50,
        10,
        RecursiveCharacterChunker::default_separators(),
    );
    let text = "Hello world.\n\nThis is a test.\n\nAnother paragraph here.";
    let chunks = chunker.chunk(text);
    assert!(!chunks.is_empty());
}

#[test]
fn test_semantic_chunker() {
    let chunker = SemanticChunker::new(SemanticChunker::default_separators());
    let text = "Paragraph one.\n\nParagraph two.\n\nParagraph three.";
    let chunks = chunker.chunk(text);
    assert_eq!(chunks.len(), 3);
}

#[test]
fn test_document_indexer() {
    let chunker: Arc<dyn ChunkStrategy> = Arc::new(FixedSizeChunker::new(100, 20));
    let indexer = DocumentIndexer::new(chunker, "test-ns");

    let doc = IndexDocument {
        id: "doc1".to_string(),
        content: "This is a test document with enough content to generate multiple chunks. "
            .repeat(10),
        title: Some("Test Document".to_string()),
        metadata: ChunkMetadata::default(),
    };

    let chunks = indexer.index_document(doc).unwrap();
    assert!(!chunks.is_empty());
    for chunk in &chunks {
        assert_eq!(chunk.metadata.namespace.as_deref(), Some("test-ns"));
        assert!(chunk.id.starts_with("doc1-"));
    }
}
