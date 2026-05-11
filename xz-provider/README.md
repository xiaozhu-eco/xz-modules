# xz-provider

> Unified LLM provider abstraction — one-liner config to switch between OpenAI, Claude, Ollama

## Features

- Multi-provider: OpenAI / Claude / Ollama (local)
- Streaming & non-streaming responses via `LlmProvider` trait
- Smart routing: capability-aware, cost-aware, fastest-latency, named routes
- Fallback chains with configurable conditions (always, rate limit, error status)
- Tool calling as first-class citizen (tool definitions, tool calls, tool results)
- Structured output (JSON mode / JSON Schema)
- Prompt caching support (Anthropic ephemeral + usage tracking)
- Token usage tracking with cache hit info
- Thinking/reasoning support (Claude extended thinking, DeepSeek reasoning, OpenAI o-series)
- Multimodal input: text, image (URL/base64), audio, file references
- Config hot-reload via `ConfigWatcher`
- Environment variable interpolation in YAML config (`${VAR_NAME:-default}`)

## Quick Start

```rust
use xz_provider::{ProviderBuilder, ProviderConfig, LlmProvider};
use xz_provider::{CompletionRequest, Message, RequestOptions, RouteContext};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router = ProviderBuilder::new()
        .with_config(ProviderConfig::from_json(r#"{
            "default_model": "gpt-4o",
            "providers": {
                "openai": {
                    "provider_type": "open_ai",
                    "api_key": "sk-...",
                    "models": [{
                        "name": "gpt-4o",
                        "capabilities": {
                            "context_window": 128000,
                            "max_output_tokens": 4096
                        }
                    }]
                }
            },
            "routing": {}
        }")?)
        .build()
        .await?;

    let response = router.complete(
        &RouteContext::default(),
        CompletionRequest::new("gpt-4o", vec![Message::user("Hello, world!")]),
        RequestOptions::default(),
    ).await?;

    println!("{}", response.content.unwrap_or_default());
    println!("Tokens: {} prompt + {} completion",
        response.usage.prompt_tokens, response.usage.completion_tokens);

    Ok(())
}
```

### Streaming

```rust
use futures::StreamExt;
use xz_provider::StreamEvent;

let mut stream = router.complete_stream(
    &RouteContext::default(),
    CompletionRequest::new("gpt-4o", vec![Message::user("Tell me a story")]),
    RequestOptions::default(),
).await?;

while let Some(event) = stream.next().await {
    match event? {
        StreamEvent::ContentDelta { delta } => print!("{}", delta),
        StreamEvent::ThinkingDelta { delta } => eprint!("[thinking] {}", delta),
        StreamEvent::Done { usage, .. } => {
            if let Some(u) = usage {
                eprintln!("\nTotal tokens: {}", u.total_tokens);
            }
        }
        _ => {}
    }
}
```

### Tool Calling

```rust
use xz_provider::{ToolDefinition, ToolChoice};
use serde_json::json;

let mut request = CompletionRequest::new("gpt-4o", vec![
    Message::user("What's the weather in Tokyo?"),
]);
request.tools = Some(vec![ToolDefinition {
    name: "get_weather".into(),
    description: "Get current weather for a city".into(),
    parameters: json!({
        "type": "object",
        "properties": {
            "city": { "type": "string" }
        },
        "required": ["city"]
    }),
    strict: None,
}]);

let response = router.complete(
    &RouteContext::default(),
    request,
    RequestOptions::default(),
).await?;

for tc in &response.tool_calls {
    println!("LLM wants to call: {} with args: {}", tc.function_name, tc.arguments);

    // Execute the tool and feed the result back
    let messages = vec![
        Message::user("What's the weather in Tokyo?"),
        Message::Assistant {
            content: xz_provider::MessageContent::None,
            tool_calls: Some(vec![tc.clone()]),
            cache_control: None,
        },
        Message::tool_result(&tc.id, "Sunny, 22\u{00b0}C"),
    ];
    // ... send follow-up request with messages
}
```

## Providers

| Provider | Feature Flag | `provider_type` value | Description |
|----------|-------------|-----------------------|-------------|
| OpenAI   | `openai` (default) | `open_ai` | GPT-4o, GPT-4, DeepSeek, Qwen, etc. |
| Claude   | `claude` | `claude` | Anthropic Claude models |
| Ollama   | `local` | `local` | Local LLMs via Ollama / llama.cpp |

## Routing

The `ProviderRouter` supports multiple routing strategies set via `RouteContext`:

| Strategy | How |
|----------|-----|
| Explicit model | `RouteContext { model: Some("gpt-4o".into()), .. }` |
| Named route | `RouteContext { named_route: Some("chat".into()), .. }` — uses pre-defined rules from config |
| Capability-aware | `RouteContext { capabilities: Some(CapabilityRequest { tool_calling: true, min_context_window: Some(128000), .. }), .. }` |
| Cost preference | `cost_preference: CostPreference::Cheapest` / `Fastest` / `Balanced` |

Fallback chains are configured per named route in JSON/YAML:

```json
"routing": {
    "chat": {
        "model": "gpt-4o",
        "provider": "openai",
        "fallback": [
            { "model": "claude-sonnet", "provider": "claude", "condition": "rate_limit_only" }
        ]
    }
}
```

## License

MIT OR Apache-2.0
