use crate::error::AgentError;

#[cfg(feature = "code-exec")]
pub async fn execute_llm_call(
    prompt: &str,
    model: &str,
    temperature: f32,
    max_tokens: u32,
) -> Result<String, AgentError> {
    use xz_provider::{
        CompletionRequest, LlmProvider, RequestOptions,
        types::message::Message,
    };

    let router = xz_provider::ProviderBuilder::new()
        .build()
        .map_err(|e| AgentError::Io(e.to_string()))?;

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

    let response = router
        .complete(request, RequestOptions::default())
        .await
        .map_err(|e| AgentError::Io(format!("LLM call failed: {}", e)))?;

    Ok(response.content.unwrap_or_default())
}

#[cfg(not(feature = "code-exec"))]
pub async fn execute_llm_call(
    prompt: &str,
    _model: &str,
    _temperature: f32,
    _max_tokens: u32,
) -> Result<String, AgentError> {
    let preview: String = prompt.chars().take(80).collect();
    Ok(format!("[LLM response for: {}...] (enable 'code-exec' feature)", preview))
}
