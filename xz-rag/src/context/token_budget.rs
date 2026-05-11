use crate::types::rag::{BuiltContext, Citation, CitationFormat};

use crate::types::retrieval::RetrievedChunk;

/// Manages token budget and assembles context from retrieved chunks.
#[derive(Debug, Clone)]
pub struct ContextBuilder {
    total_budget: usize,
    system_prompt_reserve: usize,
    query_reserve: usize,
    output_reserve: usize,
    min_chunk_overlap: usize,
    citation_format: CitationFormat,
}

impl ContextBuilder {
    pub fn new(total_budget: usize) -> Self {
        Self {
            total_budget,
            system_prompt_reserve: 256,
            query_reserve: 128,
            output_reserve: 512,
            min_chunk_overlap: 50,
            citation_format: CitationFormat::Numeric,
        }
    }

    pub fn with_reserves(
        mut self,
        system: usize,
        query: usize,
        output: usize,
    ) -> Self {
        self.system_prompt_reserve = system;
        self.query_reserve = query;
        self.output_reserve = output;
        self
    }

    pub fn with_citation_format(mut self, format: CitationFormat) -> Self {
        self.citation_format = format;
        self
    }

    /// Available tokens for context chunks.
    pub fn context_budget(&self) -> usize {
        let reserved = self.system_prompt_reserve + self.query_reserve + self.output_reserve;
        self.total_budget.saturating_sub(reserved)
    }

    /// Build context from ranked chunks, fitting within token budget.
    pub fn build(&self, chunks: &[RetrievedChunk], _query: &str) -> BuiltContext {
        let budget = self.context_budget();
        let mut context_text = String::new();
        let mut citations = Vec::new();
        let mut chunks_used = 0;
        let mut chunks_dropped = 0;
        let mut tokens_used = 0;

        // Simple token estimation: 1 token ≈ 4 chars
        let token_estimate = |text: &str| text.len() / 4;

        for (i, chunk) in chunks.iter().enumerate() {
            let chunk_tokens = token_estimate(&chunk.content);
            if tokens_used + chunk_tokens > budget {
                chunks_dropped += 1;
                continue;
            }

            let citation = Citation {
                index: i + 1,
                chunk_id: chunk.chunk_id.clone(),
                content: chunk.content.clone(),
                document_title: chunk.metadata.document_title.clone(),
                score: chunk.score,
                channel: chunk.channel.clone(),
            };

            match self.citation_format {
                CitationFormat::Numeric => {
                    context_text.push_str(&format!("[{}] {}\n", citation.index, chunk.content));
                }
                CitationFormat::ChunkId => {
                    context_text.push_str(&format!("({}) {}\n", chunk.chunk_id, chunk.content));
                }
                CitationFormat::SourceName => {
                    let source = chunk
                        .metadata
                        .document_title
                        .as_deref()
                        .unwrap_or(&chunk.chunk_id);
                    context_text.push_str(&format!("@{}: {}\n", source, chunk.content));
                }
            }

            citations.push(citation);
            chunks_used += 1;
            tokens_used += chunk_tokens;
        }

        if chunks.len() > chunks_used {
            chunks_dropped = chunks.len() - chunks_used;
        }

        BuiltContext {
            context_text,
            citations,
            chunks_used,
            chunks_dropped,
            tokens_used,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::chunk::ChunkMetadata;

    fn make_hit(id: &str, content: &str, score: f32) -> RetrievedChunk {
        RetrievedChunk {
            chunk_id: id.to_string(),
            document_id: format!("doc_{}", id),
            content: content.to_string(),
            score,
            channel: "semantic".to_string(),
            channel_score: score,
            metadata: ChunkMetadata::default(),
            embedding: None,
        }
    }

    #[test]
    fn test_context_budget_enforcement() {
        let builder = ContextBuilder::new(2000)
            .with_reserves(200, 100, 300);

        let budget = builder.context_budget();
        assert_eq!(budget, 1400); // 2000 - 600

        // 1400 tokens * 4 chars/token ≈ 5600 chars. Use 10000 to exceed.
        let long = "x".repeat(10000);
        let chunks = vec![
            make_hit("c1", &long, 0.9),
        ];

        let built = builder.build(&chunks, "test");
        // Content is too long, should be dropped
        assert_eq!(built.chunks_used, 0);
        assert_eq!(built.chunks_dropped, 1);
    }

    #[test]
    fn test_context_build_fits() {
        let builder = ContextBuilder::new(10000);
        let chunks = vec![
            make_hit("c1", "Short content A", 0.9),
            make_hit("c2", "Short content B", 0.8),
        ];

        let built = builder.build(&chunks, "test");
        assert_eq!(built.chunks_used, 2);
        assert_eq!(built.citations.len(), 2);
        assert!(built.context_text.contains("[1]"));
        assert!(built.context_text.contains("[2]"));
    }
}
