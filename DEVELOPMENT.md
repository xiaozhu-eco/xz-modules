# xz-modules 开发规范 — Development Specification

> **本文档描述 xz-modules 的所有硬性开发规则（MUST）。所有贡献者必须遵守。**
> **This document defines all hard rules (MUST) for xz-modules development. All contributors must follow them.**

See also:
- [AGENTS.md](./AGENTS.md) — AI agent core constraints and quick reference
- [CONTRIBUTING.md](./CONTRIBUTING.md) — Open-source contribution guide

---

## 目录 / Table of Contents

1. [Architecture Principles / 架构原则](#1-architecture-principles--架构原则)
2. [Five-Dimension Criteria / 五维标准](#2-five-dimension-criteria--五维标准)
3. [API Design / API 设计](#3-api-design--api-设计)
4. [Error Handling / 错误处理](#4-error-handling--错误处理)
5. [Async Patterns / 异步模式](#5-async-patterns--异步模式)
6. [Dependency Policy / 依赖策略](#6-dependency-policy--依赖策略)
7. [Versioning & Release / 版本与发版](#7-versioning--release--版本与发版)
8. [Testing Standards / 测试标准](#8-testing-standards--测试标准)

---

## 1. Architecture Principles / 架构原则

### Positioning / 定位

xz-modules 是小竹 AI 生态的基础设施仓库。它**不是**产品代码仓库。

xz-modules is the infrastructure repository for the 小竹 AI ecosystem. It is **NOT** a product codebase.

### MUST Rules / 硬性规则

- **MUST**: 仅可复用的基础能力才能加入。任何产品特定的业务逻辑、UI 组件或工作流代码不得加入。
- **MUST**: Only reusable infrastructure capabilities may be added. No product-specific business logic, UI components, or workflow code.
- **MUST**: 使用 trait 抽象实现可替换性，便于测试和 mock。
- **MUST**: Use trait abstractions for swappable implementations to facilitate testing.
- **MUST**: 遵循最小依赖原则 — 能用标准库解决的不用第三方 crate。
- **MUST**: Follow minimal dependency principle — prefer stdlib solutions over third-party crates.

### Counter-Examples / 反例

> **Bad**: 在 xz-modules 中添加 "Writing 产品的文章评分逻辑"。这是产品特定的业务逻辑。
> **Bad**: Adding "Writing product's essay scoring logic" to xz-modules. This is product-specific business logic.

> **Bad**: 在 crate 中直接使用具体实现类而非 trait 抽象，导致无法 mock 测试。
> **Bad**: Using concrete implementations instead of trait abstractions, making mocking impossible in tests.

---

## 2. Five-Dimension Criteria / 五维标准

This section defines the five dimensions for evaluating every proposed change.

### 2.1 Reusability / 复用性

#### MUST Rules / 硬性规则

| Rule | Description |
|------|-------------|
| MUST-RE-01 | The capability must serve **2+ products** without modification / 能力必须服务 **2+ 产品**且无需修改 |
| MUST-RE-02 | The problem must be **domain-independent**, not product-specific / 必须解决**领域无关**的问题，非产品特定 |
| MUST-RE-03 | Configuration must be injectable, not hardcoded / 配置必须可注入，不可硬编码 |

#### Judgment Criteria / 判断标准

- Would this feature be useful if a new product joins the ecosystem? (Yes → passes)
- 如果新加入一个产品，这个功能是否仍然有用？（是 → 通过）
- Is this tied to a specific product's data model or business process? (Yes → fails)
- 是否与特定产品的数据模型或业务流程绑定？（是 → 不通过）

#### Counter-Examples / 反例

> **Bad**: 在 xz-modules 中添加一个函数，只处理 Writing 产品的笔记格式解析。
> **Bad**: Adding a function that only parses Writing product's note format.

> **Bad**: 硬编码某个产品的 API endpoint 在基础设施 crate 中。
> **Bad**: Hardcoding a specific product's API endpoint in an infrastructure crate.

### 2.2 Interface / 接口

#### MUST Rules / 硬性规则

| Rule | Description |
|------|-------------|
| MUST-IF-01 | All public APIs must have **complete rustdoc** documentation / 所有公共 API 必须有**完整 rustdoc** 文档 |
| MUST-IF-02 | Use **trait abstractions** for swappable implementations / 使用 **trait 抽象**实现可替换 |
| MUST-IF-03 | **Do not expose** internal implementation details in public API surface / **不暴露**内部实现细节到公共 API |
| MUST-IF-04 | Public API must not contain UI components or business rules / 公共 API 不得包含 UI 组件或业务规则 |

#### Judgment Criteria / 判断标准

- Can a consumer use this API without reading the implementation source? (Docs sufficient → passes)
- 使用者是否无需阅读实现源码就能使用这个 API？（文档足够 → 通过）
- Is the API signature stable enough that changing it would be a breaking change? (Yes → good)
- API 签名是否足够稳定，变更它会被视为 breaking change？（是 → 好）

#### Counter-Examples / 反例

> **Bad**: 公开 `pub struct InternalCache { ... }`，其中包含内部实现字段。
> **Bad**: Exposing `pub struct InternalCache { ... }` with internal implementation fields.

> **Bad**: 公共函数没有 rustdoc 文档，使用者必须读源码才能理解参数含义。
> **Bad**: Public function without rustdoc, forcing users to read source code to understand parameters.

### 2.3 Performance / 性能

#### MUST Rules / 硬性规则

| Rule | Description |
|------|-------------|
| MUST-PF-01 | All I/O operations **must be async** (tokio); never block in async contexts / 所有 I/O 操作**必须异步**（tokio）；禁止在异步上下文中阻塞 |
| MUST-PF-02 | **Do not** use `std::sync::Mutex` or `std::sync::RwLock` in async code — use `tokio::sync` / **禁止**在异步代码中使用 `std::sync::Mutex`/`RwLock` — 使用 `tokio::sync` |
| MUST-PF-03 | Critical code paths **must have benchmarks**; no regressions without justification / 关键路径**必须要有 benchmark**；禁止无故性能回退 |
| MUST-PF-04 | Avoid unnecessary allocations in hot paths / 避免热路径中的不必要分配 |

#### Judgment Criteria / 判断标准

- Is there a synchronous I/O call inside an async function? (Yes → fails)
- 异步函数中是否存在同步 I/O 调用？（是 → 不通过）
- Is there a `std::sync::Mutex` or `std::sync::RwLock` in code used from async contexts? (Yes → fails)
- 在异步上下文中使用的代码是否存在 `std::sync::Mutex`/`RwLock`？（是 → 不通过）

#### Counter-Examples / 反例

> **Bad**: `let cache = Arc::new(std::sync::Mutex::new(HashMap::new()));` 在异步调度器中使用。（会导致死锁）
> **Bad**: `let cache = Arc::new(std::sync::Mutex::new(HashMap::new()));` used in an async scheduler. (Causes deadlocks)

> **Bad**: 在关键路径中使用 `format!` 或 `clone()` 而不是复用缓冲区。
> **Bad**: Using `format!` or `clone()` in hot paths instead of reusing buffers.

### 2.4 Security / 安全

#### MUST Rules / 硬性规则

| Rule | Description |
|------|-------------|
| MUST-SE-01 | **No `unsafe` code** allowed (workspace-level `forbid(unsafe_code)`) / **禁止 `unsafe` 代码**（workspace 级别禁止） |
| MUST-SE-02 | **No hardcoded secrets**, API keys, or tokens in source code / **禁止硬编码**密钥、API Key 或 Token |
| MUST-SE-03 | All public API functions **must validate inputs**; never panic on malformed data / 所有公共 API 函数**必须校验输入**；禁止对异常数据 panic |
| MUST-SE-04 | **No `.unwrap()` or `.expect()`** in library code — propagate errors via `Result` / **禁止 `.unwrap()` 和 `.expect()`** — 通过 `Result` 传播错误 |
| MUST-SE-05 | **No `.unwrap()` or `.expect()`** in library test helper code — use `?` or `.ok()` / 库测试辅助代码中也**禁止 `.unwrap()` 和 `.expect()`** — 使用 `?` 或 `.ok()` |

#### Judgment Criteria / 判断标准

- Is there any `unsafe { }` block in the code? (Yes → fails, workspace already forbids)
- 代码中是否存在 `unsafe { }` 块？（是 → 不通过，workspace 已禁止）
- Is there any `unwrap()` or `expect()` in library code? (Yes → fails)
- 库代码中是否存在 `unwrap()` 或 `expect()`？（是 → 不通过）
- Does a public function panic on empty input instead of returning `Result::Err`? (Yes → fails)
- 公共函数是否在空输入时 panic 而不是返回 `Result::Err`？（是 → 不通过）

#### Counter-Examples / 反例

> **Bad**: `let result = db_query().unwrap();` — 如果查询失败，整个进程 panic。
> **Bad**: `let result = db_query().unwrap();` — panics the entire process on query failure.

> **Bad**: `pub fn parse_config(s: &str) -> Config { s.parse().expect("invalid config"); }` — 输入无效时 panic。
> **Bad**: `pub fn parse_config(s: &str) -> Config { s.parse().expect("invalid config"); }` — panics on invalid input.

### 2.5 Dependencies / 依赖

#### MUST Rules / 硬性规则

| Rule | Description |
|------|-------------|
| Must-DP-01 | All dependency versions **must be declared** in workspace `[workspace.dependencies]` / 所有依赖版本**必须在** workspace `[workspace.dependencies]` 中声明 |
| MUST-DP-02 | **Must not** add dependencies with licenses incompatible with MIT or Apache-2.0 / **禁止**添加许可证不兼容 MIT/Apache-2.0 的依赖 |
| MUST-DP-03 | **Must not** add unmaintained or abandoned crates / **禁止**未维护或已废弃的 crate |
| MUST-DP-04 | **Minimize** new dependency count; prefer stdlib solutions when feasible / **最小化**新增依赖数量；可行时优先使用标准库 |

#### Judgment Criteria / 判断标准

- Can this functionality be implemented with stdlib or existing dependencies? (Yes → don't add new dep)
- 这个功能能否用标准库或已有依赖实现？（是 → 不加新依赖）
- Is the crate's latest release > 2 years old? (Yes → likely abandoned, verify carefully)
- 该 crate 的最新发布是否超过 2 年？（是 → 可能已废弃，需仔细验证）
- Is the crate's license GPL/AGPL? (Yes → fails, incompatible with MIT/Apache-2.0)
- 该 crate 的许可证是否为 GPL/AGPL？（是 → 不通过，与 MIT/Apache-2.0 不兼容）

#### Counter-Examples / 反例

> **Bad**: 每个 crate 单独声明 `serde = "1"` 而非使用 workspace 统一版本管理。
> **Bad**: Each crate declares `serde = "1"` independently instead of using workspace-level version management.

> **Bad**: 添加一个 3 年未更新的 crate 来实现简单的字符串操作，而标准库已有对应功能。
> **Bad**: Adding a crate unmaintained for 3 years to implement simple string operations that stdlib already supports.

---

## 3. API Design / API 设计

### Stability Commitment / 稳定性承诺

所有公共 API 遵循语义化版本控制（Semver）。破坏性变更（breaking changes）只能在 major 版本发布时引入。

All public APIs follow Semantic Versioning. Breaking changes may only be introduced in major releases.

### Breaking Change Definition / 破坏性变更定义

以下变更视为 breaking change：

- Removing or renaming a public type, trait, function, or module
- Changing a function signature (adding required params, changing param types)
- Adding a required trait bound to an existing trait
- Changing the behavior of an existing function in a way that breaks consumers

### Naming Conventions / 命名约定

- **Types/Traits**: `CamelCase` (e.g., `EmbeddingStore`, `SearchEngine`)
- **Functions/Variables**: `snake_case` (e.g., `generate_embedding`, `search_results`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `MAX_BATCH_SIZE`)
- **Error types**: `Error` suffix (e.g., `MemoryError`, `ProviderError`)
- **Trait methods**: Use descriptive verb phrases (e.g., `fn embed(&self, text: &str) -> Result<Vec<f32>>`)

### Error Type Pattern / 错误类型模式

每个 crate 必须定义一个公共错误类型，使用 `thiserror` 派生：

```rust
#[derive(Debug, thiserror::Error)]
pub enum MyCrateError {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("invalid input: {0}")]
    InvalidInput(String),
    
    #[error("operation timed out")]
    Timeout,
}
```

---

## 4. Error Handling / 错误处理

### MUST Rules / 硬性规则

- **MUST**: 使用 `thiserror` 定义 crate 级错误枚举。
- **MUST**: Use `thiserror` for crate-level error enums.
- **MUST**: 提供 `is_retryable()` 方法区分可重试和不可重试错误。
- **MUST**: Provide `is_retryable()` method to distinguish retryable vs non-retryable errors.
- **MUST**: 库代码中完全禁止 `.unwrap()` 和 `.expect()`。
- **MUST**: Absolutely no `.unwrap()` or `.expect()` in library code.
- **MUST**: 禁止在 async 上下文中进行 panic 处理 — 一律使用 `Result`。
- **MUST**: Never panic in async contexts — always use `Result`.

### Error Type Pattern / 错误类型模式

```rust
pub enum MemoryError {
    #[error("storage error: {0}")]
    Storage(String),
    
    #[error("not found: {key}")]
    NotFound { key: String },
    
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl MemoryError {
    /// Returns true if the operation can be safely retried.
    pub fn is_retryable(&self) -> bool {
        matches!(self, MemoryError::Storage(_))
    }
}
```

---

## 5. Async Patterns / 异步模式

### MUST Rules / 硬性规则

- **MUST**: 所有 I/O 操作使用 `tokio` async。禁止在 async 函数中调用同步 I/O。
- **MUST**: All I/O must use `tokio` async. No synchronous I/O in async functions.
- **MUST NOT**: 禁止在 async 上下文中使用 `std::sync::Mutex` 或 `std::sync::RwLock`。
- **MUST NOT**: No `std::sync::Mutex` or `std::sync::RwLock` in async contexts.
- **MUST**: 使用 `tokio::sync::Mutex`、`tokio::sync::RwLock` 或 `tokio::sync::Semaphore`。
- **MUST**: Use `tokio::sync::Mutex`, `tokio::sync::RwLock`, or `tokio::sync::Semaphore` instead.
- **MUST**: 长时间运行的任务必须支持取消和超时。
- **MUST**: Long-running tasks must support cancellation and timeout.

### Counter-Examples / 反例

> **Bad**: `std::sync::Mutex` in async context — 持有锁时执行 `.await` 可能导致死锁。
> **Bad**: `std::sync::Mutex` in async context — holding a lock across `.await` points causes deadlocks.

> **Bad**: `tokio::task::spawn_blocking(|| std::thread::sleep(Duration::from_secs(30)))` — 不必要地占用 blocking thread pool。
> **Bad**: Unnecessarily occupying the blocking thread pool.

### Correct Pattern / 正确模式

```rust
use tokio::sync::RwLock;

pub struct Cache {
    inner: RwLock<HashMap<String, Value>>,
}

impl Cache {
    pub async fn get(&self, key: &str) -> Option<Value> {
        self.inner.read().await.get(key).cloned()
    }
    
    pub async fn set(&self, key: String, value: Value) {
        self.inner.write().await.insert(key, value);
    }
}
```

---

## 6. Dependency Policy / 依赖策略

### Adding a New Dependency / 新增依赖流程

1. **Justify**: Can stdlib or existing dependencies do this? Document why not.
2. **Check license**: Must be compatible with MIT OR Apache-2.0 (no GPL/AGPL).
3. **Check maintenance**: Latest release < 2 years ago, repo has recent commits.
4. **Add to workspace**: Declare in `[workspace.dependencies]` in workspace `Cargo.toml`.
5. **Use in crate**: Reference via `{ workspace = true }` in crate's `Cargo.toml`.

### MUST Rules / 硬性规则

- **MUST**: 所有依赖版本在 workspace `Cargo.toml` 中统一声明。
- **MUST**: All dependency versions declared in workspace `Cargo.toml`.
- **MUST**: Crate 通过 `{ workspace = true }` 引用 workspace 依赖。
- **MUST**: Crates reference workspace dependencies via `{ workspace = true }`.
- **MUST NOT**: 各 crate 独立声明不同版本的同一依赖。
- **MUST NOT**: Different crates declaring different versions of the same dependency.

### Recommended Tools / 推荐工具

```bash
# Check for dependency issues
cargo deny check

# Audit dependencies for security vulnerabilities
cargo audit

# Show dependency tree
cargo tree
```

### Dependency Topology / 依赖拓扑

`xz-provider` depends on `xz-auth-client` and `xz-auth-core` from an external workspace. These crates are resolved from:

- `../../xz-auth/crates/xz-auth-client`
- `../../xz-auth/crates/xz-auth-core`

This layout requires the following local development directory structure:

```text
<parent>/
├── xz-auth/
└── xz-modules/
```

#### MUST Rules / 硬性规则

- **MUST**: `xz-auth` and `xz-modules` must be sibling directories for local development.
- **MUST**: `xz-provider` must keep the external workspace path layout above.
- **MUST**: Treat this as a required workspace arrangement for local development; do not assume a standalone `xz-modules` checkout can resolve these crates.
- **MUST NOT**: Change these dependencies to git or registry sources in this document.

---

## 7. Versioning & Release / 版本与发版

### Semver Policy / 语义化版本策略

- **Major**: Breaking changes to public API
- **Minor**: New features, deprecations, non-breaking additions
- **Patch**: Bug fixes, performance improvements, documentation

### Changelog / 变更日志

每个版本发布时必须维护 `CHANGELOG.md`，遵循 [Keep a Changelog](https://keepachangelog.com/) 格式：

```markdown
# Changelog

## [0.2.0] - 2024-01-15

### Added
- New feature X with trait abstraction
- Support for Y provider

### Changed
- Deprecated Z function in favor of W (scheduled for removal in 0.3.0)

### Fixed
- Corrected SSE streaming fragmentation handling
```

### Release Process / 发布流程

1. 更新 workspace `Cargo.toml` 版本号
2. 更新所有需要发布 crate 的版本号（crates.io 要求版本号递增）
3. 更新 `CHANGELOG.md`
4. 创建 Git tag: `v<version>`
5. CI 自动推送到 crates.io (见 `.github/workflows/publish.yml`)

### Publish Order / 发布顺序

发布按依赖层级进行（中间有 90 秒延时以避免 crates.io 限流）：

- **Layer 0** (无内部依赖): xz-provider, xz-embed, xz-search, xz-rerank, xz-knowledge-graph
- **Layer 1** (依赖 Layer 0): xz-memory, xz-skill
- **Layer 2** (依赖 Layer 0-1): xz-rag, xz-agent

---

## 8. Testing Standards / 测试标准

### MUST Rules / 硬性规则

| Type | Requirement |
|------|-------------|
| Unit tests | New features must include unit tests covering happy path, error cases, and edge cases |
| Regression tests | Bug fixes must include a test that fails before the fix |
| Doc tests | Public APIs should include doc test examples (`/// ``` ``` `) |
| Build | `cargo build --workspace --all-features` must pass |
| Lint | `cargo clippy --workspace --all-targets --all-features -- -D warnings` must pass |
| Format | `cargo fmt --all -- --check` must pass |

### Benchmark Requirements / Benchmark 要求

- 关键路径（如搜索、嵌入生成）必须包含 benchmark 测试
- Benchmark 使用 `criterion` crate
- 性能回退必须有明确的解释和 justification

### Test Organization / 测试组织

```rust
// Unit tests: inline in each module
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_happy_path() { ... }
    
    #[test]
    fn test_error_case() { ... }
    
    #[test]
    fn test_edge_case() { ... }
}

// Integration tests: in tests/ directory
// tests/integration_test.rs
```

### CI Gates / CI 门禁

All four CI jobs must pass before any PR can be merged:

1. Format: `cargo fmt --all -- --check`
2. Lint: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
3. Test: `cargo test --workspace --all-features`
4. Doc: `cargo doc --workspace --all-features --no-deps`
