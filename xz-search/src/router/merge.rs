use std::collections::HashMap;

use crate::types::{SearchConfig, SearchItem, SearchResult};

/// 结果融合排序
pub fn merge_and_sort(
    mut items: Vec<SearchItem>,
    config: &SearchConfig,
    interleave_sources: bool,
) -> SearchResult {
    let total_results = items.len() as u64;

    // 按 score 降序排序
    items.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    // 如果启用交叉来源
    let sorted = if interleave_sources {
        interleave_by_source(items)
    } else {
        items
    };

    // 截断到 max_results
    let truncated: Vec<SearchItem> = sorted.into_iter().take(config.max_results).collect();

    SearchResult {
        query: String::new(), // 由调用方填充
        items: truncated,
        total_results,
        latency_ms: 0,
        cached: false,
        engines_used: vec![],
        rewritten_query: None,
    }
}

/// 来源交叉分布：防止同一引擎的结果占据所有前排
pub fn interleave_by_source(items: Vec<SearchItem>) -> Vec<SearchItem> {
    let mut by_source: HashMap<String, Vec<SearchItem>> = HashMap::new();
    let mut source_order: Vec<String> = Vec::new();

    for item in items {
        let source = item.source.clone();
        if !by_source.contains_key(&source) {
            source_order.push(source.clone());
        }
        by_source.entry(source).or_default().push(item);
    }

    // 每个来源队列内按分数降序
    for queue in by_source.values_mut() {
        queue.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    }

    let mut result = Vec::new();
    let mut had_more = true;

    while had_more {
        had_more = false;
        for source in &source_order {
            if let Some(queue) = by_source.get_mut(source) {
                if !queue.is_empty() {
                    result.push(queue.remove(0));
                    had_more = true;
                }
            }
        }
    }

    result
}
