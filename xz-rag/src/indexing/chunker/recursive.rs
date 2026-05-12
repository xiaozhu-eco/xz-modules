use super::ChunkStrategy;

/// Recursive character chunker (LangChain-style).
///
/// Splits text by trying separators in order, from largest to smallest.
pub struct RecursiveCharacterChunker {
    chunk_size: usize,
    overlap: usize,
    separators: Vec<String>,
}

impl RecursiveCharacterChunker {
    pub fn new(chunk_size: usize, overlap: usize, separators: Vec<String>) -> Self {
        assert!(overlap < chunk_size, "overlap must be < chunk_size");
        Self {
            chunk_size,
            overlap,
            separators,
        }
    }

    /// Default separators: paragraphs, lines, sentences, spaces.
    pub fn default_separators() -> Vec<String> {
        vec![
            "\n\n".to_string(),
            "\n".to_string(),
            ". ".to_string(),
            " ".to_string(),
        ]
    }
}

impl ChunkStrategy for RecursiveCharacterChunker {
    fn chunk(&self, text: &str) -> Vec<String> {
        self.split_recursive(text, &self.separators)
    }

    fn name(&self) -> &str {
        "recursive"
    }
}

impl RecursiveCharacterChunker {
    fn split_recursive(&self, text: &str, separators: &[String]) -> Vec<String> {
        if text.len() <= self.chunk_size {
            return if text.is_empty() {
                vec![]
            } else {
                vec![text.to_string()]
            };
        }

        if let Some((sep, rest)) = separators.split_first() {
            let parts: Vec<&str> = text.split(sep).collect();
            if parts.len() > 1 {
                let mut chunks = Vec::new();
                for part in parts {
                    chunks.extend(self.split_recursive(part, rest));
                }
                return self.merge_chunks(chunks);
            }
            return self.split_recursive(text, rest);
        }

        // Fallback: character-level split
        let chars: Vec<char> = text.chars().collect();
        let mut chunks = Vec::new();
        let mut start = 0;

        while start < chars.len() {
            let end = (start + self.chunk_size).min(chars.len());
            chunks.push(chars[start..end].iter().collect());
            start = start + self.chunk_size - self.overlap;
        }
        chunks
    }

    fn merge_chunks(&self, chunks: Vec<String>) -> Vec<String> {
        if chunks.is_empty() {
            return vec![];
        }

        let mut merged = Vec::new();
        let mut current = chunks[0].clone();

        for next in chunks.iter().skip(1) {
            if current.len() + next.len() <= self.chunk_size {
                current.push_str(next);
            } else {
                merged.push(current);
                current = next.clone();
            }
        }
        merged.push(current);
        merged
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recursive_chunker() {
        let chunker = RecursiveCharacterChunker::new(50, 10, RecursiveCharacterChunker::default_separators());
        let text = "Hello world.\n\nThis is a test.\n\nAnother paragraph here.";
        let chunks = chunker.chunk(text);
        assert!(!chunks.is_empty());
        for c in &chunks {
            assert!(c.len() <= 60); // chunk_size + overlap tolerance
        }
    }
}
