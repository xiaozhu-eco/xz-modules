use crate::error::AgentError;
use crate::executor::dag::ExecutionContext;
use crate::types::step::AgentAction;

mod llm;
mod web_search;
mod report;
mod condition;

pub use llm::execute_llm_call;
pub use web_search::execute_web_search;
pub use report::execute_report;
pub use condition::evaluate_condition;

pub async fn execute_action(
    action: &AgentAction,
    ctx: &ExecutionContext,
) -> Result<String, AgentError> {
    match action {
        AgentAction::LlmCall { prompt_template, model, temperature, max_tokens } => {
            let prompt = ctx.resolve_template(prompt_template);
            execute_llm_call(&prompt, model, *temperature, *max_tokens).await
        }
        AgentAction::WebSearch { query_template, sources, max_results } => {
            let query = ctx.resolve_template(query_template);
            execute_web_search(&query, sources, *max_results).await
        }
        AgentAction::SkillInvoke { skill_id, input_template } => {
            let input = ctx.resolve_template(input_template);
            Ok(format!("Skill '{}' invoked with: {}", skill_id, input))
        }
        AgentAction::WebExtract { url_template, .. } => {
            let url = ctx.resolve_template(url_template);
            execute_web_extract(&url).await
        }
        AgentAction::GenerateReport { format, template } => {
            let title = template.as_deref().unwrap_or("Untitled Report");
            execute_report(title, format)
        }
        AgentAction::Notify { title_template, body_template, .. } => {
            let title = ctx.resolve_template(title_template);
            let body = ctx.resolve_template(body_template);
            tracing::info!(target: "xz_agent", title = %title, body = %body, "agent_notification");
            Ok(format!("Notified: {}", title))
        }
        AgentAction::CodeBlock { code, language, .. } => {
            let resolved_code = ctx.resolve_template(code);
            Ok(format!("Executed {} code block ({} chars)", language, resolved_code.len()))
        }
        AgentAction::MemoryRecall { query_template, .. } => {
            let query = ctx.resolve_template(query_template);
            Ok(format!("Memory recalled for: {}", query))
        }
        AgentAction::Condition { expression, .. } => {
            if evaluate_condition(expression, ctx) {
                Ok("condition: true".into())
            } else {
                Err(AgentError::StepFailed {
                    step: "condition".into(),
                    reason: format!("Condition '{}' evaluated to false", expression),
                })
            }
        }
    }
}

#[cfg(feature = "web-extract")]
async fn execute_web_extract(url: &str) -> Result<String, AgentError> {
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| AgentError::Io(e.to_string()))?;
    let text = response
        .text()
        .await
        .map_err(|e| AgentError::Io(e.to_string()))?;
    let truncated: String = text.chars().take(4096).collect();
    Ok(truncated)
}

#[cfg(not(feature = "web-extract"))]
async fn execute_web_extract(url: &str) -> Result<String, AgentError> {
    Ok(format!("[Web extract from: {}] (enable 'web-extract' feature)", url))
}
