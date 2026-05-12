use xz_agent::*;

#[tokio::main]
async fn main() {
    let scheduler = InMemoryAgentScheduler::new(SchedulerConfig::default());

    // Build a multi-step pipeline: search → extract → report
    let search_step = AgentStep::new(
        "search",
        "Web search",
        AgentAction::WebSearch {
            query_template: "latest Rust release".into(),
            sources: vec![],
            max_results: 5,
        },
    );

    let mut extract_step = AgentStep::new(
        "extract",
        "Extract content",
        AgentAction::WebExtract {
            url_template: "https://example.com".into(),
            selector: None,
        },
    );
    extract_step.depends_on = vec!["search".to_string()];

    let mut report_step = AgentStep::new(
        "report",
        "Generate report",
        AgentAction::GenerateReport {
            format: ReportFormat::Markdown,
            template: Some(
                "Search: {{ steps.search.output }}\nExtract: {{ steps.extract.output }}"
                    .into(),
            ),
        },
    );
    report_step.depends_on = vec!["extract".to_string()];

    let agent = Agent::new(
        "research-agent",
        "Research Pipeline Agent",
        AgentTrigger::manual(),
    )
    .with_steps(vec![search_step, extract_step, report_step]);

    scheduler.register(agent).await.unwrap();
    println!("Registered research-agent with 3-step pipeline");

    let result = scheduler.trigger("research-agent", None).await.unwrap();
    println!("Run ID: {}", result.run_id);
    println!("Success: {}", result.success);
    println!("Steps completed: {:?}", result.steps_completed);
    println!("Steps failed: {:?}", result.steps_failed);
    println!("Output: {:?}", result.output);

    for sr in &result.step_results {
        let status = if sr.success { "V" } else { "X" };
        println!(
            "  {} {}: {:?} ({}ms, {} retries)",
            status, sr.step_id, sr.output, sr.duration_ms, sr.retries
        );
    }
}
