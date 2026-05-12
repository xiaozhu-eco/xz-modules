use xz_rag::{RetrieveRequest, StructuredFilter};

#[test]
fn test_retrieve_request_builder() {
    let request = RetrieveRequest::builder("test query")
        .top_k(5)
        .namespace("my-ns")
        .build();

    assert_eq!(request.query, "test query");
    assert_eq!(request.top_k, 5);
    assert_eq!(request.namespace, Some("my-ns".to_string()));
}

#[test]
fn test_structured_filter() {
    let filter = StructuredFilter::MetadataEq {
        key: "author".to_string(),
        value: "Alice".to_string(),
    };

    match &filter {
        StructuredFilter::MetadataEq { key, value } => {
            assert_eq!(key, "author");
            assert_eq!(value, "Alice");
        }
        _ => panic!("expected MetadataEq"),
    }
}

#[test]
fn test_query_preprocessing() {
    let pp = xz_rag::QueryPreprocessing::QueryExpansion { count: 3 };
    match pp {
        xz_rag::QueryPreprocessing::QueryExpansion { count } => assert_eq!(count, 3),
        _ => panic!("expected QueryExpansion"),
    }
}
