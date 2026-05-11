use std::collections::HashMap;

use crate::near_dup::NearDuplicateDetector;
use crate::router::DedupStrategy;
use crate::types::SearchItem;

fn normalize_url(url: &str) -> String {
    let mut u = url.trim().to_lowercase();
    while u.ends_with('/') {
        u.pop();
    }
    u = u
        .replace("https://www.", "https://")
        .replace("http://www.", "http://");
    if let Some(pos) = u.find('#') {
        u.truncate(pos);
    }
    u
}

pub fn deduplicate_by_url(items: Vec<SearchItem>, strategy: &DedupStrategy) -> Vec<SearchItem> {
    match strategy {
        DedupStrategy::UrlExact => dedup_exact(items),
        DedupStrategy::UrlExactWithNearDup { threshold } => {
            let exact_deduped = dedup_exact(items);
            dedup_near_duplicates(exact_deduped, *threshold)
        }
    }
}

fn dedup_exact(items: Vec<SearchItem>) -> Vec<SearchItem> {
    let mut seen: HashMap<String, SearchItem> = HashMap::new();

    for item in items {
        let normalized = normalize_url(&item.url);
        seen.entry(normalized)
            .and_modify(|existing| {
                if item.score > existing.score {
                    existing.source = format!("{}, {}", existing.source, item.source);
                    existing.score = item.score;
                }
            })
            .or_insert(item);
    }

    let mut result: Vec<SearchItem> = seen.into_values().collect();
    result.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    result
}

fn dedup_near_duplicates(items: Vec<SearchItem>, threshold: f32) -> Vec<SearchItem> {
    if items.len() <= 1 {
        return items;
    }

    let detector = NearDuplicateDetector::new(64, threshold);
    let mut kept: Vec<SearchItem> = Vec::new();
    let mut signatures: Vec<Vec<u64>> = Vec::new();

    for item in items {
        let content = format!("{} {}", item.title, item.snippet);
        let sig = detector.compute_signature(&content);

        let is_dup = signatures
            .iter()
            .any(|existing| detector.is_near_duplicate(&sig, existing));

        if !is_dup {
            signatures.push(sig);
            kept.push(item);
        }
    }

    kept
}
