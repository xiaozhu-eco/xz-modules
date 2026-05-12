use std::collections::HashMap;
use xz_embed::*;

#[test]
fn test_metadata_filter_combinators() {
    let filter = MetadataFilter::and([
        MetadataFilter::eq("source", "docs"),
        MetadataFilter::in_values("lang", &["rust", "go"]),
        MetadataFilter::or([
            MetadataFilter::eq("status", "active"),
            MetadataFilter::eq("status", "draft"),
        ]),
    ]);

    match &filter {
        MetadataFilter::And(inner) => assert_eq!(inner.len(), 3),
        _ => panic!("expected And"),
    }
}

#[test]
fn test_metadata_filter_not() {
    let filter = MetadataFilter::not(MetadataFilter::eq("status", "archived"));

    match &filter {
        MetadataFilter::Not(inner) => match inner.as_ref() {
            MetadataFilter::Eq { key, value } => {
                assert_eq!(key, "status");
                assert_eq!(value, "archived");
            }
            _ => panic!("expected Eq"),
        },
        _ => panic!("expected Not"),
    }
}

#[test]
fn test_filter_from_str_auto() {
    let f = MetadataFilter::eq("key", "val");
    match f {
        MetadataFilter::Eq { key, value } => {
            assert_eq!(key, "key");
            assert_eq!(value, "val");
        }
        _ => panic!(),
    }
}
