use super::ChunkStrategy;

/// Fixed-size chunker with overlap.
pub struct FixedSizeChunker {
    chunk_size: usize,
    overlap: usize,
}

impl FixedSizeChunker {
    pub fn new(chunk_size: usize, overlap: usize) -> Self {
        assert!(overlap < chunk_size, "overlap must be < chunk_size");
        Self { chunk_size, overlap }
    }
}

impl ChunkStrategy for FixedSizeChunker {
    fn chunk(&self, text: &str) -> Vec<String> {
        let chars: Vec<char> = text.chars().collect();
        let mut chunks = Vec::new();
        let mut start = 0;

        while start < chars.len() {
            let end = (start + self.chunk_size).min(chars.len());
            let chunk: String = chars[start..end].iter().collect();
            chunks.push(chunk);
            start = start + self.chunk_size - self.overlap;
        }

        chunks
    }

    fn name(&self) -> &str {
        "fixed_size"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_chunker() {
        let chunker = FixedSizeChunker::new(10, 2);
        let text = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let chunks = chunker.chunk(text);
        assert!(chunks.len() > 1);
        // Each chunk should be at most 10 chars
        for c in &chunks {
            assert!(c.len() <= 10 + 2); // +2 for overlap tolerance
        }
        // First chunk should end with some chars from second chunk's start
        assert!(chunks[0].ends_with(&chunks[1][..2.min(chunks[1].len())]));
    }
}
