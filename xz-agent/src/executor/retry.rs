use std::time::Duration;

use crate::error::AgentError;
use crate::types::result::StepResult;
use crate::types::step::AgentStep;

/// Execute a step with retry logic, respecting max_retries and backoff.
pub async fn execute_with_retry<F, Fut>(
    step: &AgentStep,
    mut action: F,
) -> StepResult
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<String, AgentError>>,
{
    let mut attempt = 0;
    let start = std::time::Instant::now();

    loop {
        match action().await {
            Ok(output) => {
                return StepResult::success(
                    &step.id,
                    Some(output),
                    start.elapsed().as_millis() as u64,
                );
            }
            Err(e) => {
                attempt += 1;

                if attempt > step.max_retries {
                    return StepResult::failure(
                        &step.id,
                        e.to_string(),
                        start.elapsed().as_millis() as u64,
                        attempt,
                    );
                }

                let backoff = Duration::from_millis(
                    step.retry_backoff_ms * 2_u64.pow(attempt - 1),
                );
                tokio::time::sleep(backoff).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_retry_success_first_try() {
        let step = AgentStep::new(
            "test",
            "test",
            crate::types::step::AgentAction::Notify {
                method: crate::types::step::NotificationMethod::InApp,
                title_template: "t".into(),
                body_template: "b".into(),
            },
        );

        let result = execute_with_retry(&step, || async { Ok("done".to_string()) }).await;
        assert!(result.success);
        assert_eq!(result.retries, 0);
    }

    #[tokio::test]
    async fn test_retry_exhausted() {
        let step = AgentStep::new(
            "test",
            "test",
            crate::types::step::AgentAction::Notify {
                method: crate::types::step::NotificationMethod::InApp,
                title_template: "t".into(),
                body_template: "b".into(),
            },
        )
        .with_retry(2, 10);

        let mut call_count = 0;
        let result = execute_with_retry(&step, || {
            call_count += 1;
            async move {
                Err(AgentError::StepFailed {
                    step: "test".into(),
                    reason: "always fail".into(),
                })
            }
        })
        .await;

        assert!(!result.success);
        assert_eq!(result.retries, 3); // initial + 2 retries
    }
}
