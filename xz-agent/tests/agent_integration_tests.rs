use xz_agent::*;

fn make_notify_step(id: &str, title: &str) -> AgentStep {
    AgentStep::new(
        id,
        format!("step {}", id),
        AgentAction::Notify {
            method: NotificationMethod::InApp,
            title_template: title.to_string(),
            body_template: String::new(),
        },
    )
}

fn make_llm_step(id: &str, prompt: &str) -> AgentStep {
    AgentStep::new(
        id,
        format!("step {}", id),
        AgentAction::LlmCall {
            prompt_template: prompt.to_string(),
            model: "mock".to_string(),
            temperature: 0.7,
            max_tokens: 100,
        },
    )
}

#[tokio::test]
async fn test_register_and_trigger_simple_agent() {
    let config = SchedulerConfig::default();
    let scheduler = InMemoryAgentScheduler::new(config);

    let step = make_notify_step("s1", "Hello");
    let agent = Agent::new("agent-1", "Test Agent", AgentTrigger::manual())
        .with_steps(vec![step]);

    scheduler.register(agent).await.unwrap();

    let result = scheduler.trigger("agent-1", None).await.unwrap();
    assert!(result.success);
    assert_eq!(result.agent_id, "agent-1");
    assert_eq!(result.steps_completed, vec!["s1"]);
    assert!(result.step_results.len() == 1);
    assert!(result.step_results[0].success);
    assert!(result.step_results[0].output.as_ref().unwrap().contains("Hello"));
}

#[tokio::test]
async fn test_register_and_trigger_linear_pipeline() {
    let config = SchedulerConfig::default();
    let scheduler = InMemoryAgentScheduler::new(config);

    let step1 = AgentStep::new(
        "s1",
        "step 1",
        AgentAction::Notify {
            method: NotificationMethod::InApp,
            title_template: "Step 1".into(),
            body_template: String::new(),
        },
    );

    let mut step2 = AgentStep::new(
        "s2",
        "step 2",
        AgentAction::LlmCall {
            prompt_template: "Process: {{ steps.s1.output }}".into(),
            model: "mock".to_string(),
            temperature: 0.7,
            max_tokens: 100,
        },
    );
    step2.depends_on = vec!["s1".to_string()];

    let agent = Agent::new("agent-2", "Pipeline Agent", AgentTrigger::manual())
        .with_steps(vec![step1, step2]);

    scheduler.register(agent).await.unwrap();

    let result = scheduler.trigger("agent-2", None).await.unwrap();
    assert!(result.success);
    assert_eq!(result.steps_completed, vec!["s1", "s2"]);
    assert_eq!(result.step_results.len(), 2);
    assert!(result.step_results[0].output.as_ref().unwrap().contains("Step 1"));
}

#[tokio::test]
async fn test_trigger_not_found() {
    let scheduler = InMemoryAgentScheduler::new(SchedulerConfig::default());

    let result = scheduler.trigger("nonexistent", None).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), AgentError::NotFound(_)));
}

#[tokio::test]
async fn test_list_agents() {
    let scheduler = InMemoryAgentScheduler::new(SchedulerConfig::default());

    scheduler
        .register(
            Agent::new("a1", "Agent One", AgentTrigger::manual())
                .with_steps(vec![make_notify_step("s1", "test")]),
        )
        .await
        .unwrap();

    scheduler
        .register(
            Agent::new("a2", "Agent Two", AgentTrigger::cron("0 8 * * *", "UTC"))
                .with_steps(vec![make_llm_step("s1", "hello")]),
        )
        .await
        .unwrap();

    let filter = AgentFilter::default();
    let agents = scheduler.list(&filter).await.unwrap();
    assert_eq!(agents.len(), 2);
}

#[tokio::test]
async fn test_pause_and_resume() {
    let scheduler = InMemoryAgentScheduler::new(SchedulerConfig::default());

    scheduler
        .register(
            Agent::new("a1", "Test", AgentTrigger::manual())
                .with_steps(vec![make_notify_step("s1", "test")]),
        )
        .await
        .unwrap();

    scheduler.pause("a1").await.unwrap();
    scheduler.resume("a1").await.unwrap();
}

#[tokio::test]
async fn test_unregister() {
    let scheduler = InMemoryAgentScheduler::new(SchedulerConfig::default());

    scheduler
        .register(
            Agent::new("a1", "Test", AgentTrigger::manual())
                .with_steps(vec![make_notify_step("s1", "test")]),
        )
        .await
        .unwrap();

    scheduler.unregister("a1").await.unwrap();

    let filter = AgentFilter::default();
    let agents = scheduler.list(&filter).await.unwrap();
    assert!(agents.is_empty());
}

#[tokio::test]
async fn test_trigger_batch() {
    let scheduler = InMemoryAgentScheduler::new(SchedulerConfig::default());

    scheduler
        .register(
            Agent::new("a1", "A1", AgentTrigger::manual())
                .with_steps(vec![make_notify_step("s1", "test")]),
        )
        .await
        .unwrap();

    scheduler
        .register(
            Agent::new("a2", "A2", AgentTrigger::manual())
                .with_steps(vec![make_notify_step("s1", "test")]),
        )
        .await
        .unwrap();

    let results = scheduler.trigger_batch(&["a1", "a2"]).await.unwrap();
    assert_eq!(results.len(), 2);
    assert!(results[0].success);
    assert!(results[1].success);
}
