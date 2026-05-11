# xz-agent

Agent scheduling system with DAG execution and multi-trigger support (cron, events, manual).

## Features

- **DAG-based execution** — steps with dependency ordering, parallel layer execution, cycle detection
- **Multi-trigger** — cron, interval, event (with filter matching), manual, and webhook
- **Rich step actions** — LLM calls, web search, web extraction, skill invocation, code blocks, reporting, notifications, memory recall, conditional branching
- **Retry with exponential backoff** — per-step configurable retries
- **On-failure strategies** — Abort, Skip, Retry, Fallback to alternate step
- **Template resolution** — `{{ steps.<id>.output }}` and `{{ variables.<key> }}` interpolation between steps
- **Pagination & filtering** — list agents by enabled/trigger type with offset/limit

## Feature flags

| Flag | Description | Adds |
|---|---|---|
| `code-exec` | LLM provider integration + WASM code sandbox | `xz-provider`, `wasmtime` |
| `web-search` | Web search via search router | `xz-search` |
| `web-extract` | Fetch and extract web page content | `reqwest` |
| `skill-integration` | Invoke external skills | `xz-skill` |
| `system-notify` | OS-level desktop notifications | `notify-rust` |

## Quick start

```rust
use xz_agent::*;

#[tokio::main]
async fn main() {
    let scheduler = InMemoryAgentScheduler::new(SchedulerConfig::default());

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

    scheduler.register(agent).await.unwrap();
    let result = scheduler.trigger("hello-agent", None).await.unwrap();

    println!("Run {}: {}", result.run_id, if result.success { "OK" } else { "FAIL" });
    for sr in &result.step_results {
        println!("  {} → success={}, {}ms", sr.step_id, sr.success, sr.duration_ms);
    }
}
```

### Multi-step pipeline with DAG

```rust
let search = AgentStep::new("search", "Web search", AgentAction::WebSearch {
    query_template: "latest Rust release".into(),
    sources: vec![],
    max_results: 5,
});

let extract = AgentStep::new("extract", "Extract content", AgentAction::WebExtract {
    url_template: "https://example.com".into(),
    selector: None,
}).depends_on("search");

let report = AgentStep::new("report", "Generate report", AgentAction::GenerateReport {
    format: ReportFormat::Markdown,
    template: Some("Search: {{ steps.search.output }}\nExtract: {{ steps.extract.output }}".into()),
}).depends_on("extract");

let agent = Agent::new("research-agent", "Research Pipeline", AgentTrigger::manual())
    .with_steps(vec![search, extract, report]);

scheduler.register(agent).await.unwrap();
let result = scheduler.trigger("research-agent", None).await.unwrap();
```

## API overview

### Agent

```rust
let agent = Agent::new("id", "Name", AgentTrigger::cron("0 8 * * *", "UTC"))
    .with_steps(vec![step_a, step_b])
    .with_config(AgentConfig { max_execution_time_secs: 600, ..Default::default() });
```

### Triggers

```rust
AgentTrigger::manual()
AgentTrigger::cron("0 */6 * * *", "Asia/Shanghai")
AgentTrigger::interval(3600)          // every hour
AgentTrigger::Event { event_type: "webhook.received".into(), filter: None }
AgentTrigger::Webhook { path: "/hooks/my-agent".into(), secret: Some("key".into()) }
```

### Step actions

| Action | Description |
|---|---|
| `LlmCall` | Chat completion with model, temperature, max_tokens |
| `WebSearch` | Aggregated search with query template and source filtering |
| `WebExtract` | Fetch a URL, optionally extract via CSS selector |
| `SkillInvoke` | Call an external skill by ID with templated input |
| `GenerateReport` | Render output as Markdown, HTML, JSON, or PlainText |
| `Notify` | In-app, desktop, email, or webhook notifications |
| `CodeBlock` | Execute code in a sandbox (requires `code-exec`) |
| `MemoryRecall` | Query agent memory for relevant context |
| `Condition` | Branch execution: `then` / `else` step lists |

### Scheduler trait

```rust
scheduler.register(agent).await?;
scheduler.unregister("id").await?;
scheduler.trigger("id", Some("input")).await?;
scheduler.trigger_batch(&["a", "b"]).await?;
scheduler.start().await?;
scheduler.stop().await?;
scheduler.list(&AgentFilter::default()).await?;
scheduler.get_status("id").await?;
scheduler.cancel("run-id").await?;
scheduler.pause("id").await?;
scheduler.resume("id").await?;
```

## Error handling

Errors are retryable-aware via `AgentError`:

```rust
match error {
    AgentError::NotFound(id) => { /* agent doesn't exist */ }
    AgentError::StepFailed { step, reason } => { /* step execution failed */ }
    AgentError::CircularDependency(ids) => { /* cycle in step DAG */ }
    AgentError::Timeout(secs) => { /* execution timed out */ }
    AgentError::ConcurrencyLimit { max } => { /* too many concurrent runs */ }
    _ => { /* ... */ }
}
```

Check `AgentError::is_retryable()` for transient vs permanent failures.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
