use super::ChunkStrategy;

/// Semantic chunker that splits on separator boundaries.
pub struct SemanticChunker {
    separators: Vec<String>,
}

impl SemanticChunker {
    pub fn new(separators: Vec<String>) -> Self {
        Self { separators }
    }

    pub fn default_separators() -> Vec<String> {
        vec!["\n\n".to_string(), "\n".to_string(), ". ".to_string()]
    }
}

impl ChunkStrategy for SemanticChunker {
    fn chunk(&self, text: &str) -> Vec<String> {
        for sep in &self.separators {
            let parts: Vec<&str> = text.split(sep).collect();
            if parts.len() > 1 {
                return parts
                    .into_iter()
                    .filter(|p| !p.trim().is_empty())
                    .map(|p| p.to_string())
                    .collect();
            }
        }
        // Fallback: return whole text
        if text.is_empty() {
            vec![]
        } else {
            vec![text.to_string()]
        }
    }

    fn name(&self) -> &str {
        "semantic"
    }
}
