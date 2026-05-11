use crate::types::rag::Citation;

/// Format citations according to the chosen style.
pub fn format_citations_numeric(citations: &[Citation]) -> String {
    citations
        .iter()
        .map(|c| {
            format!(
                "[{}] {}{}",
                c.index,
                c.content.chars().take(100).collect::<String>(),
                if c.content.len() > 100 { "..." } else { "" }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Extract citation references from generated answer text.
pub fn extract_citation_indices(answer: &str) -> Vec<usize> {
    let mut indices = Vec::new();
    let mut current = String::new();
    let mut in_bracket = false;

    for ch in answer.chars() {
        match (ch, in_bracket) {
            ('[', false) => {
                in_bracket = true;
                current.clear();
            }
            (']', true) => {
                if let Ok(idx) = current.parse::<usize>() {
                    indices.push(idx);
                }
                in_bracket = false;
            }
            (c, true) if c.is_ascii_digit() || c == ',' => {
                current.push(c);
            }
            _ => {
                in_bracket = false;
                current.clear();
            }
        }
    }

    indices
}
