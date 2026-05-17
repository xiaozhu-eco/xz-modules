use std::collections::HashMap;
use tokio::sync::RwLock;

use crate::error::AgentError;
use crate::executor::dag::{topological_sort, ExecutionContext};
use crate::executor::retry::execute_with_retry;
use crate::scheduler::config::SchedulerConfig;
use crate::traits::AgentScheduler;
use crate::types::agent::Agent;
use crate::types::result::{AgentRunResult, StepResult, TokenUsage};
use crate::types::status::{AgentFilter, AgentStatus, UpsertResult};

/// In-memory agent scheduler (for testing).
#[derive(Debug)]
pub struct InMemoryAgentScheduler {
    agents: RwLock<HashMap<String, Agent>>,
    statuses: RwLock<HashMap<String, AgentStatus>>,
    running: RwLock<bool>,
    #[allow(dead_code)]
    config: SchedulerConfig,
}

impl InMemoryAgentScheduler {
    pub fn new(config: SchedulerConfig) -> Self {
        Self {
            agents: RwLock::new(HashMap::new()),
            statuses: RwLock::new(HashMap::new()),
            running: RwLock::new(false),
            config,
        }
    }

    async fn execute_steps(&self, agent: &Agent, _input: Option<&str>) -> Result<AgentRunResult, AgentError> {
        let run_id = uuid::Uuid::new_v4().to_string();
        let started_at = current_epoch_ms();
        let mut ctx = ExecutionContext::new();

        // Handle Condition steps: expand inline
        let steps = expand_conditions(&agent.steps, &ctx);

        let layers = topological_sort(&steps)?;
        let mut step_results: Vec<StepResult> = Vec::new();
        let mut all_success = true;

        for layer in &layers {
            let handles: Vec<_> = layer
                .iter()
                .map(|step| {
                    execute_step(step.clone(), ctx.clone())
                })
                .collect();

            let results = futures::future::join_all(handles).await;

            for (step, result) in layer.iter().zip(results) {
                ctx.set_step_output(&step.id, result.output.clone().unwrap_or_default());

                if !result.success {
                    all_success = false;
                    match &step.on_failure {
                        crate::types::step::OnFailure::Abort => {
                            step_results.push(result);
                            return Ok(AgentRunResult {
                                run_id,
                                agent_id: agent.id.clone(),
                                success: false,
                                started_at,
                                completed_at: current_epoch_ms(),
                                output: None,
                                error: Some(format!("Step {} failed", step.id)),
                                steps_completed: step_results.iter().map(|s| s.step_id.clone()).collect(),
                                steps_failed: vec![step.id.clone()],
                                token_usage: TokenUsage::default(),
                                step_results,
                            });
                        }
                        crate::types::step::OnFailure::Skip => {
                            step_results.push(result);
                            continue;
                        }
                        crate::types::step::OnFailure::Retry => {
                            step_results.push(result);
                            continue;
                        }
                        crate::types::step::OnFailure::Fallback { step_id: fallback_id } => {
                            // TODO: execute fallback step and use its output
                            tracing::warn!(
                                "Step {} failed, falling back to step {}",
                                step.id,
                                fallback_id
                            );
                            step_results.push(result);
                            continue;
                        }
                    }
                }

                step_results.push(result);
            }
        }

        let output = ctx.step_outputs.values().last().cloned();

        Ok(AgentRunResult {
            run_id,
            agent_id: agent.id.clone(),
            success: all_success,
            started_at,
            completed_at: current_epoch_ms(),
            output,
            error: if all_success { None } else { Some("some steps failed".into()) },
            steps_completed: step_results.iter().filter(|s| s.success).map(|s| s.step_id.clone()).collect(),
            steps_failed: step_results.iter().filter(|s| !s.success).map(|s| s.step_id.clone()).collect(),
            token_usage: TokenUsage::default(),
            step_results,
        })
    }
}

#[async_trait::async_trait]
impl AgentScheduler for InMemoryAgentScheduler {
    async fn register(&self, agent: Agent) -> Result<UpsertResult, AgentError> {
        let existed = self.agents.write().await.insert(agent.id.clone(), agent).is_some();
        if existed {
            Ok(UpsertResult::Updated)
        } else {
            Ok(UpsertResult::Created)
        }
    }

    async fn unregister(&self, id: &str) -> Result<(), AgentError> {
        self.agents.write().await.remove(id);
        self.statuses.write().await.remove(id);
        Ok(())
    }

    async fn trigger(&self, id: &str, input: Option<&str>) -> Result<AgentRunResult, AgentError> {
        let agent = self
            .agents
            .read()
            .await
            .get(id)
            .cloned()
            .ok_or_else(|| AgentError::NotFound(id.to_string()))?;

        self.execute_steps(&agent, input).await
    }

    async fn trigger_batch(&self, ids: &[&str]) -> Result<Vec<AgentRunResult>, AgentError> {
        let mut results = Vec::new();
        for id in ids {
            results.push(self.trigger(id, None).await?);
        }
        Ok(results)
    }

    async fn start(&self) -> Result<(), AgentError> {
        *self.running.write().await = true;
        Ok(())
    }

    async fn stop(&self) -> Result<(), AgentError> {
        *self.running.write().await = false;
        Ok(())
    }

    async fn list(&self, filter: &AgentFilter) -> Result<Vec<Agent>, AgentError> {
        let agents = self.agents.read().await;
        let mut result: Vec<Agent> = agents.values().cloned().collect();

        if filter.enabled_only {
            result.retain(|a| a.enabled);
        }
        if let Some(ref trigger_type) = filter.trigger_type {
            result.retain(|a| std::mem::discriminant(&a.trigger) == std::mem::discriminant(trigger_type));
        }

        result.sort_by_key(|a| std::cmp::Reverse(a.updated_at));

        let start = filter.page.offset.min(result.len());
        let end = (start + filter.page.limit).min(result.len());
        Ok(result[start..end].to_vec())
    }

    async fn get_status(&self, id: &str) -> Result<AgentStatus, AgentError> {
        self.statuses
            .read()
            .await
            .get(id)
            .cloned()
            .ok_or_else(|| AgentError::NotFound(id.to_string()))
    }

    async fn cancel(&self, run_id: &str) -> Result<(), AgentError> {
        let _ = run_id;
        Err(AgentError::Cancelled("not implemented in memory scheduler".into()))
    }

    async fn pause(&self, id: &str) -> Result<(), AgentError> {
        self.statuses
            .write()
            .await
            .insert(id.to_string(), AgentStatus::Paused);
        Ok(())
    }

    async fn resume(&self, id: &str) -> Result<(), AgentError> {
        self.statuses
            .write()
            .await
            .insert(id.to_string(), AgentStatus::Idle);
        Ok(())
    }
}

// === Helpers ===

fn current_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Expand Condition steps into concrete branches.
fn expand_conditions(
    steps: &[crate::types::step::AgentStep],
    _ctx: &ExecutionContext,
) -> Vec<crate::types::step::AgentStep> {
    let mut result = Vec::new();
    for step in steps {
        if let crate::types::step::AgentAction::Condition {
            expression: _,
            then,
            r#else,
        } = &step.action
        {
            // Default: take the "then" branch (simplified - no expression evaluation yet)
            result.extend(then.clone());
            let _ = r#else;
        } else {
            result.push(step.clone());
        }
    }
    result
}

async fn execute_step(
    step: crate::types::step::AgentStep,
    ctx: ExecutionContext,
) -> StepResult {
    let timeout_secs = step.timeout_secs;
    let step_meta = step.clone();

    let future = execute_with_retry(&step_meta, move || {
        let step = step.clone();
        let ctx = ctx.clone();
        async move {
            crate::action::execute_action(&step.action, &ctx).await
        }
    });

    match tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), future).await {
        Ok(result) => result,
        Err(_elapsed) => StepResult::failure(
            &step_meta.id,
            format!("step timed out after {}s", timeout_secs),
            timeout_secs.saturating_mul(1000),
            0,
        ),
    }
}
