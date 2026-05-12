use crate::error::AgentError;

#[cfg(feature = "web-search")]
pub async fn execute_web_search(
    query: &str,
    _sources: &[String],
    _max_results: usize,
) -> Result<String, AgentError> {
    use xz_search::{SearchRouter, SearchConfig, SearchOptions};

    let mut router = SearchRouter::new();
    let config = SearchConfig::default();
    let options = SearchOptions::default();

    let result = router
        .aggregated_search(query, &config, &options)
        .await
        .map_err(|e| AgentError::Io(format!("Search failed: {}", e)))?;

    let summary: Vec<String> = result
        .items
        .iter()
        .take(5)
        .map(|item| format!("- {} ({})", item.title, item.url))
        .collect();

    Ok(summary.join("\n"))
}

#[cfg(not(feature = "web-search"))]
pub async fn execute_web_search(
    query: &str,
    _sources: &[String],
    _max_results: usize,
) -> Result<String, AgentError> {
    Ok(format!("[Search results for: {}] (enable 'web-search' feature)", query))
}
