use futures::stream::Stream;

use crate::error::RagError;
use crate::types::rag::RagStreamEvent;

/// Create a streaming generation channel.
///
/// Returns a stream of `RagStreamEvent` values.
#[allow(dead_code)]
pub fn create_stream(
    _prompt: &str,
) -> impl Stream<Item = Result<RagStreamEvent, RagError>> {
    // Placeholder: in production, uses xz-provider streaming
    futures::stream::iter(vec![
        Ok(RagStreamEvent::GenerationStarted {
            context_chunks: 0,
            context_tokens: 0,
        }),
        Ok(RagStreamEvent::ContentDelta {
            delta: "Streaming placeholder".to_string(),
        }),
        Ok(RagStreamEvent::Done {
            total_latency_ms: 0,
            citations: vec![],
            usage: crate::types::rag::RagTokenUsage::default(),
        }),
    ])
}
