use xz_search::*;

#[test]
fn test_url_dedup() {
    let items = vec![
        SearchItem {
            title: "Result 1".into(),
            url: "https://example.com/a".into(),
            snippet: "snippet 1".into(),
            source: "mock1".into(),
            published_at: None,
            score: 0.9,
            domain: "example.com".into(),
            detected_language: None,
            extracted_content: None,
        },
        SearchItem {
            title: "Result 2".into(),
            url: "https://example.com/a".into(),
            snippet: "snippet 2".into(),
            source: "mock2".into(),
            published_at: None,
            score: 0.6,
            domain: "example.com".into(),
            detected_language: None,
            extracted_content: None,
        },
        SearchItem {
            title: "Result 3".into(),
            url: "https://example.com/b".into(),
            snippet: "snippet 3".into(),
            source: "mock1".into(),
            published_at: None,
            score: 0.7,
            domain: "example.com".into(),
            detected_language: None,
            extracted_content: None,
        },
    ];

    let deduped = xz_search::router::deduplicate_by_url(items, &DedupStrategy::UrlExact);
    assert_eq!(deduped.len(), 2); // duplicate removed
    assert_eq!(deduped[0].score, 0.9); // highest score kept first
}

#[test]
fn test_url_normalization() {
    let items = vec![
        SearchItem {
            title: "A".into(),
            url: "https://example.com/page/".into(),
            snippet: "a".into(),
            source: "mock1".into(),
            published_at: None,
            score: 0.9,
            domain: "example.com".into(),
            detected_language: None,
            extracted_content: None,
        },
        SearchItem {
            title: "B".into(),
            url: "https://example.com/page".into(),
            snippet: "b".into(),
            source: "mock2".into(),
            published_at: None,
            score: 0.8,
            domain: "example.com".into(),
            detected_language: None,
            extracted_content: None,
        },
    ];

    let deduped = xz_search::router::deduplicate_by_url(items, &DedupStrategy::UrlExact);
    assert_eq!(deduped.len(), 1); // trailing slash normalized
}
