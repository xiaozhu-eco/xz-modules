# xz-skill

Skill plugin system — registration, execution, and WebAssembly sandbox for AI agent tool-calling.

## Features

| Feature (flag)         | Description                                                  | Default      |
|------------------------|--------------------------------------------------------------|-------------|
| `wasm-runtime`         | Execute tools as compiled WebAssembly modules via wasmtime   | **on**      |
| `http-tool`            | Invoke external HTTP APIs as skill tools via reqwest         | off         |
| `sqlite-registry`      | Persist skill registrations in SQLite via sqlx               | off         |
| `hot-reload`           | Watch the skills directory and auto-reload on changes        | off         |

## Architecture

```
┌──────────────┐     ┌──────────────────┐
│  SkillRegistry│◄────│  FileSkillRegistry│  (YAML directory layout)
│  (trait)      │     │  SqliteSkillRegistry│ (SQLite, feature-gated)
└──────┬───────┘     └──────────────────┘
       │
┌──────▼───────┐     ┌──────────────────┐
│  SkillRuntime │◄────│ DefaultSkillRuntime│  (LLM prompt injection + tool-calling loop)
│  (trait)      │     │ WasmRuntime         │  (WebAssembly sandbox, feature-gated)
└──────┬───────┘     │ HttpToolExecutor    │  (HTTP tool calls, feature-gated)
       │             └──────────────────┘
┌──────▼───────┐
│  Pipeline    │  Multi-skill chaining with sequencing, conditions, and parallel fan-out
└──────────────┘
```

## Quick start

```rust
use xz_skill::{
    DefaultSkillRuntime, ExecutionContext, FileSkillRegistry,
    SandboxConfig, SkillFilter, SkillRegistry, SkillRuntime,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create a file-based registry from a skills directory
    let registry = FileSkillRegistry::new("./skills".as_ref()).await?;

    // 2. Register a skill from YAML
    let skill_yaml = r#"
id: echo-skill
name: Echo Skill
version: 1.0.0
description: Echoes back the user input
author: org.xiaozhu
enabled: true
prompt: You are an echo assistant. Repeat the user's input back to them.
permissions: []
tools:
  - name: echo
    description: Echo back the input
    input_schema:
      type: object
      properties:
        input:
          type: string
      required: [input]
    tool_type:
      type: builtin
      handler: echo
min_agent_version: ">=0.1.0"
"#;

    let skill: xz_skill::Skill = serde_yaml::from_str(skill_yaml)?;
    let result = registry.register(skill).await?;
    println!("Register result: {:?}", result);

    // 3. List enabled skills
    let filter = SkillFilter {
        enabled_only: true,
        ..Default::default()
    };
    let skills = registry.list(&filter).await?;
    for s in &skills {
        println!("  {} v{} — {} ({})", s.name, s.version, s.description, s.id);
    }

    // 4. Create the default runtime backed by the registry
    let runtime = DefaultSkillRuntime::new(Some(std::sync::Arc::new(registry)))
        .with_permissions(false, vec![]);

    // 5. Build execution context
    let context = ExecutionContext {
        user_id: "user-1".into(),
        session_id: "session-1".into(),
        messages: vec![],
        provider: None,
        search: None,
        memory: None,
    };

    // 6. Execute the skill
    let output = runtime.execute("echo-skill", "Hello, World!", &context).await?;
    println!("Output: {}", output.content);
    println!("Duration: {}ms", output.duration_ms);

    Ok(())
}
```

## Skill layout (file-based registry)

```
skills/
└── echo-skill/
    ├── skill.yaml          # Skill definition
    └── echo.wasm           # Optional WASM module
```

### `skill.yaml`

```yaml
id: echo-skill
name: Echo Skill
version: 1.0.0
description: Echoes back the user input
author: org.xiaozhu
enabled: true
prompt: You are an echo assistant. Repeat the user's input back to them.
permissions: []
tools:
  - name: echo
    description: Echo back the input
    input_schema:
      type: object
      properties:
        input:
          type: string
      required: [input]
    tool_type:
      type: builtin
      handler: echo
min_agent_version: ">=0.1.0"
```

## Tool types

### Builtin

Built-in handlers run in-process with no external dependencies. Available handlers: `echo`, `now`, `uuid`, `json_path`, `base64_encode`, `base64_decode`.

```yaml
tool_type:
  type: builtin
  handler: echo
```

### WASM (requires `wasm-runtime`)

Sandboxed WebAssembly execution via wasmtime with fuel metering and per-instance memory limits.

```rust
use xz_skill::{WasmConfig, WasmRuntime};

let config = WasmConfig {
    memory_limit_mb: 128,
    default_timeout_ms: 10_000,
    max_instances: 5,
};
let wasm = WasmRuntime::new(config)?;

// let module = std::fs::read("echo.wasm")?;
// let result = wasm.execute(&module, "process", serde_json::json!({"input": "data"})).await?;
```

```yaml
tool_type:
  type: wasm
  module_path: echo.wasm
  memory_limit_mb: 128
  timeout_ms: 10000
```

### HTTP (requires `http-tool`)

Forward tool calls to external HTTP APIs.

```yaml
tool_type:
  type: http
  url: https://api.example.com/translate
  method: POST
  headers:
    Authorization: Bearer ${TOKEN}
  timeout_ms: 5000
```

## Permissions

Skills declare required permissions; the runtime validates them before execution:

```rust
use xz_skill::SkillPermission;

// Skill must be registered with:
permissions: [Network, FileRead]
```

| Permission   | Description                     |
|-------------|---------------------------------|
| `Network`   | Outbound HTTP requests          |
| `FileRead`  | Read access to allowed paths    |
| `FileWrite` | Write access to allowed paths   |
| `Execute`   | Shell/process execution         |
| `Custom(t)` | Application-defined permission  |

## Pipeline

Chain multiple skills with sequencing, conditions, and parallel fan-out:

```rust
use xz_skill::{PipelineStep, SkillPipeline};

let pipeline = SkillPipeline {
    steps: vec![
        PipelineStep::Skill { id: "extract".into() },
        PipelineStep::Condition {
            field: "sentiment".into(),
            op: "==".into(),
            value: "positive".into(),
            then: vec![PipelineStep::Skill { id: "celebrate".into() }],
            else_: vec![PipelineStep::Skill { id: "escalate".into() }],
        },
    ],
};
```

## Feature flags

```toml
[dependencies]
xz-skill = { version = "0.1", features = ["wasm-runtime", "http-tool"] }
```

| Feature           | Adds                                                     |
|-------------------|----------------------------------------------------------|
| `wasm-runtime`    | `WasmRuntime`, `WasmConfig` — WebAssembly sandbox        |
| `http-tool`       | `HttpToolExecutor` — HTTP-based tool calls               |
| `sqlite-registry` | `SqliteSkillRegistry` — SQLite-backed persistence         |
| `hot-reload`      | `FileSkillRegistry::watch()` — filesystem watcher         |

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
