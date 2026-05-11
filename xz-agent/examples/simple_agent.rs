use xz_agent::*;

#[tokio::main]
async fn main() {
    // Create a scheduler
    let scheduler = InMemoryAgentScheduler::new(SchedulerConfig::default());

    // Define a simple agent with one notification step
    let step = AgentStep::new(
        "notify",
        "Send notification",
        AgentAction::Notify {
            method: NotificationMethod::InApp,
            title_template: "Agent executed!".into(),
            body_template: "The agent has completed successfully.".into(),
        },
    );

    let agent = Agent::new("hello-agent", "Hello World Agent", AgentTrigger::manual())
        .with_steps(vec![step]);

    // Register and trigger
    scheduler.register(agent).await.unwrap();
    println!("Registered agent: hello-agent");

    let result = scheduler.trigger("hello-agent", None).await.unwrap();
    println!("Run ID: {}", result.run_id);
    println!("Success: {}", result.success);
    println!("Steps completed: {:?}", result.steps_completed);
    println!("Output: {:?}", result.output);

    for sr in &result.step_results {
        println!(
            "  Step {}: success={}, output={:?}, duration={}ms",
            sr.step_id, sr.success, sr.output, sr.duration_ms
        );
    }

    // List agents
    let agents = scheduler.list(&AgentFilter::default()).await.unwrap();
    println!("\nRegistered agents: {}", agents.len());
    for a in &agents {
        println!("  - {} ({})", a.name, a.id);
    }
}
