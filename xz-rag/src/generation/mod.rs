pub mod stream;

use crate::error::RagError;
use crate::types::rag::RagTokenUsage;

#[cfg(feature = "llm-generation")]
use {
    std::sync::Arc,
    xz_provider::{
        CompletionRequest, LlmProvider, RequestOptions,
        types::message::Message,
    },
};

#[cfg(feature = "llm-generation")]
pub async fn generate_response(
    provider: &Arc<dyn LlmProvider>,
    prompt: &str,
    model: Option<&str>,
    temperature: Option<f32>,
    max_tokens: Option<usize>,
) -> Result<(String, RagTokenUsage), RagError> {
    let request = CompletionRequest {
        model: model.map(|s| s.to_string()),
        messages: vec![Message::user(prompt)],
        temperature,
        max_tokens,
        stop: None,
        frequency_penalty: None,
        presence_penalty: None,
        tools: None,
        tool_choice: None,
        response_format: None,
        max_completion_tokens: None,
        top_p: None,
        top_k: None,
        seed: None,
        reasoning_effort: None,
        logprobs: None,
        logit_bias: None,
        stream_include_usage: None,
        request_id: String::new(),
    };

    let response = provider
        .complete(request, RequestOptions::default())
        .await
        .map_err(|e| RagError::Provider(format!("LLM generation failed: {}", e)))?;

    let content = response.content.unwrap_or_default();
    let usage = RagTokenUsage {
        context_tokens: 0,
        prompt_tokens: response.usage.prompt_tokens as usize,
        completion_tokens: response.usage.completion_tokens as usize,
        total_tokens: response.usage.total_tokens as usize,
        chunks_used: 0,
        chunks_dropped: 0,
    };

    Ok((content, usage))
}

#[cfg(not(feature = "llm-generation"))]
pub async fn generate_response(
    _prompt: &str,
    _model: Option<&str>,
    _temperature: Option<f32>,
    _max_tokens: Option<usize>,
) -> Result<(String, RagTokenUsage), RagError> {
    Ok((
        "This is a placeholder RAG response. Enable 'llm-generation' feature for real LLM integration.".to_string(),
        RagTokenUsage {
            context_tokens: 0,
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
            chunks_used: 0,
            chunks_dropped: 0,
        },
    ))
}
