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
9. [Linting Enforcement & CI / 静态检查与 CI 增强](#9-linting-enforcement--ci--静态检查与-ci-增强)
10. [Workspace Dependency Execution / Workspace 依赖执行细则](#10-workspace-dependency-execution--workspace-依赖执行细则)
11. [Feature Flag Naming / Feature 命名规范](#11-feature-flag-naming--feature-命名规范)
12. [Crate Structure Template / Crate 结构模板](#12-crate-structure-template--crate-结构模板)
13. [Concurrent Multi-Product Development / 多产品并发开发规范](#13-concurrent-multi-product-development--多产品并发开发规范)

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

---

## 9. Linting Enforcement & CI / 静态检查与 CI 增强

### Clippy Lint Configuration / Clippy 配置

在 workspace `Cargo.toml` 的 `[workspace.lints.clippy]` 中开启以下 lints：

```toml
[workspace.lints.clippy]
# 禁止 unwrap / expect（需配合测试代码 allow）
unwrap_used = "deny"
expect_used = "deny"
```

**注意**：`unwrap_used` / `expect_used` 会同时影响测试代码。测试代码中合理使用 `.unwrap()` 需要通过 `#![allow(clippy::unwrap_used)]` 显式豁免（在 `tests/` 目录或 `#[cfg(test)]` 模块顶部）。

### MUST Rules / 硬性规则

| Rule | Description |
|------|-------------|
| MUST-LT-01 | Every crate MUST include `[lints] workspace = true` in `Cargo.toml` to inherit workspace lint policies |
| MUST-LT-02 | `cargo clippy --workspace --all-targets --all-features -- -D warnings` MUST pass before every commit |
| MUST-LT-03 | Do NOT use `#[allow(clippy::...)]` to suppress lint warnings without a `// SAFETY:` or equivalent justification comment |
| MUST-LT-04 | New crates MUST have `#![deny(missing_docs)]` in `lib.rs` |

### Custom CI Checks / 自定义 CI 检查

除标准 Rust CI 外，建议增加以下自定义检查：

1. **unwrap/expect audit**: 扫描 `src/` 目录中的 `.unwrap()` 和 `.expect()`，排除 `#[cfg(test)]` 块和 `///` 文档注释中的调用
2. **std::sync audit**: 扫描 `src/` 目录中的 `std::sync::Mutex` 和 `std::sync::RwLock`，标记出现在 `async fn` 体内的调用
3. **Workspace dep compliance**: 解析 crate `Cargo.toml`，标记已在 `[workspace.dependencies]` 中声明但未使用 `{ workspace = true }` 的依赖

---

## 10. Workspace Dependency Execution / Workspace 依赖执行细则

### MUST Rules / 硬性规则

| Rule | Description |
|------|-------------|
| MUST-WD-01 | Every external dependency declared in `[workspace.dependencies]` MUST be referenced via `{ workspace = true }` in crate `Cargo.toml` — **never** declare a version string redundantly |
| MUST-WD-02 | Internal workspace crates (e.g. `xz-provider`, `xz-embed`) MUST be referenced via `{ workspace = true }` — the workspace root declares both `version` and `path` for each |
| MUST-WD-03 | Extra features beyond workspace defaults use `{ workspace = true, features = ["extra"] }` |
| MUST-WD-04 | New dependencies MUST be added to `[workspace.dependencies]` first, then referenced in crate `Cargo.toml` via `{ workspace = true }` |
| MUST-WD-05 | Crate-specific `[lints]` sections MUST include `workspace = true` to inherit from `[workspace.lints]` |

### Example / 示例

```toml
# ❌ WRONG — declares version that already exists in workspace
[dependencies]
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["rt", "sync"] }

# ✅ CORRECT — uses workspace reference
[dependencies]
serde = { workspace = true }
tokio = { workspace = true, features = ["rt", "sync"] }

# ✅ CORRECT — internal crate reference
[dependencies]
xz-provider = { workspace = true, optional = true }

# ✅ CORRECT — lint inheritance
[lints]
workspace = true
```

### Feature Drift Prevention / Feature 漂移预防

| Dependency | Workspace Default | Common Drift | Risk |
|---|---|---|---|
| `tokio` | `["rt", "sync", "macros", "time"]` | Some crates use `["full"]` (bloat), some drop `"time"` | **HIGH** |
| `uuid` | `["v4", "v7", "serde"]` | Missing `"serde"` → UUIDs won't serialize | **HIGH** |
| `reqwest` | `["json"]` | Missing `"json"`, adding `"stream"` | **MEDIUM** |

If a crate needs different features, the reasons must be documented in the crate's `Cargo.toml` as a comment.

---

## 11. Feature Flag Naming / Feature 命名规范

### MUST Rules / 硬性规则

| Rule | Description |
|------|-------------|
| MUST-FF-01 | Feature names MUST use `kebab-case` |
| MUST-FF-02 | Feature names MUST describe functional capability, not provider/backend name (except where the feature IS the backend switch) |
| MUST-FF-03 | Feature names MUST be consistent across crates that share the same concept |
| MUST-FF-04 | `test-utils` is a reserved feature name across ALL crates — used for exposing in-memory test implementations |

### Naming Convention by Category / 分类命名约定

| Category | Convention | Example |
|---|---|---|
| Backend/provider selection | `<backend-name>` (lowercase) | `openai`, `claude`, `ollama` |
| Capability toggles | `<capability-name>` (kebab-case) | `web-search`, `code-exec`, `vector-memory` |
| Storage backends | `store-<backend>` | `store-sqlite`, `store-postgres` |
| Integration features | `<system>-integration` | `skill-integration`, `wasm-runtime` |
| Test utilities | `test-utils` | `test-utils` |

### Counter-Examples / 反例

> **Bad**: `enableCodeExec`（camelCase）→ 应使用 `code-exec`
> **Bad**: `serpapi`（仅后端名，未体现功能）→ 应使用 `search-serpapi` 或在 feature 文档中说明
> **Bad**: Feature 名称与另一个 crate 中同名 feature 含义不同

---

## 12. Crate Structure Template / Crate 结构模板

### MUST Rules / 硬性规则

| Rule | Description |
|------|-------------|
| MUST-CS-01 | Every crate MUST have a `lib.rs` with `//!` module-level documentation and `#![deny(missing_docs)]` |
| MUST-CS-02 | Every crate MUST have a `tests/` directory with at least one integration test |
| MUST-CS-03 | Every crate MUST have a `thiserror`-derived error enum in `src/error.rs` |
| MUST-CS-04 | Every crate MUST have a `CHANGELOG.md` (updated on each release) |
| MUST-CS-05 | Public API types, traits, and functions MUST live in modules re-exported from `lib.rs` |

### Standard Crate Layout / 标准目录结构

```
xz-name/
├── Cargo.toml          # [lints] workspace = true, deps via { workspace = true }
├── CHANGELOG.md
├── README.md
├── src/
│   ├── lib.rs          # //! crate docs, #![deny(missing_docs)], pub mod + pub use
│   ├── error.rs        # #[derive(Debug, thiserror::Error)] pub enum CrateError
│   ├── traits.rs       # #[async_trait] pub trait CoreTrait
│   ├── types/
│   │   └── mod.rs      # pub struct Config, etc.
│   ├── ...             # implementation modules
│   └── .../
├── tests/
│   └── integration.rs  # Integration tests
├── examples/
│   └── basic.rs        # Minimal usage example
└── benches/
    └── benchmark.rs     # Critical path benchmarks
```

### `Cargo.toml` Template / 模板

```toml
[package]
name = "xz-name"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Short description of what this crate provides"

[dependencies]
# External — MUST use { workspace = true }
serde = { workspace = true }
tokio = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
async-trait = { workspace = true }
# Internal workspace crates — MUST use { workspace = true }
xz-provider = { workspace = true, optional = true }

[dev-dependencies]
tokio = { workspace = true, features = ["rt", "macros"] }

[features]
default = ["basic-impl"]
basic-impl = []
advanced-impl = []
test-utils = []

[lints]
workspace = true
```

---

## 13. Concurrent Multi-Product Development / 多产品并发开发规范

### Overview / 概述

xz-modules 是小竹生态中所有产品（Writing、Chat、Memo、Drive、Message 等）的共享基础设施。当多个产品团队同时需要修改 xz-modules 时，必须遵循以下协议以避免冲突、保证稳定性。

xz-modules is the shared foundation for ALL 小竹 products. When multiple product teams need to modify xz-modules concurrently, the following protocol MUST be followed to avoid conflicts and maintain stability.

### 13.1 Concurrent Modification Protocol / 并发修改协议

#### Step 1: Impact Assessment / 影响评估

**修改前必须完成**：

1. **确定影响范围**：变更涉及哪些 crate？哪些已有 public API 受影响？
2. **分类变更类型**：
   - **Non-breaking**: 新增 feature、新增 trait method（带默认实现）、新增类型、bug fix（不改变行为）
   - **Breaking**: 移除/重命名 public API、改变函数签名、增删 trait bound、改变已有函数行为
3. **确定影响产品**：哪些 小竹 产品依赖受影响的 crate？
4. **评估时间线**：各产品团队计划何时发布？变更需要多久完成？

> 请在 PR 描述中使用以下模板
> Use the following template in PR description:
> ```
> ## Impact Assessment / 影响评估
> - **Crates affected**: xz-provider, xz-memory
> - **Change type**: Non-breaking
> - **Products affected**: Writing (uses chat()), Chat (uses streaming)
> - **Product release timelines**: Writing v2.1 on 2026-03-01, Chat v1.5 on 2026-02-28
> ```

#### Step 2: Coordination / 协调

| 变更类型 | 协调要求 |
|---|---|
| **Non-breaking, 单个 crate** | 正常 PR 流程。PR 描述中注明影响评估即可 |
| **Non-breaking, 多个 crate** | 需要至少一个其他产品团队的 **review approval** |
| **Breaking, 单个 crate** | 需要**所有受影响产品团队**的 review approval + **deprecation plan** |
| **New crate** | 需要确认 **2+ 产品** 会使用此 crate + **所有产品团队**的 review |

#### Step 3: Feature Gating for Experimental Changes / 实验性变更的 Feature 门控

当某个产品需要一个新功能，但不确定其他产品是否需要时：
- **MUST**: 将新功能放在 feature flag 之后（默认关闭）
- **MUST**: Feature flag 命名为描述功能能力而非产品名（如 `web-search` 而非 `writing-feature`）
- **SHOULD**: 在 PR 中注明哪些产品计划使用此 feature

```toml
# ✅ CORRECT: capability-based feature flag
[features]
default = []
web-search = ["xz-search", "xz-provider"]  # Used by Writing, planned for Chat

# ❌ WRONG: product-specific feature
[features]
writing-feature = ["xz-search"]  # 不要用产品名命名
```

#### Step 4: Breaking Change Protocol / 破坏性变更协议

Breaking changes 需要遵循以下流程：

```
Product A needs breaking change
  │
  ├─ 1. 创建 issue 描述变更内容和理由
  ├─ 2. 通知所有受影响产品团队（通过 issue @mention 或内部频道）
  ├─ 3. 在 PR 中实现 deprecated 版本（保留旧 API + #[deprecated]）
  ├─ 4. 等待至少一个 minor release 周期（让产品迁移）
  ├─ 5. 在 CHANGELOG 中标注 deprecated + 移除计划
  ├─ 6. 下一个 major release 中移除旧 API
  └─ 7. 确认所有产品已迁移后 merge
```

**MUST Rules / 硬性规则**：

| Rule | Description |
|------|-------------|
| MUST-BC-01 | Breaking changes MUST be preceded by a `#[deprecated]` annotation with migration guidance |
| MUST-BC-02 | At least one minor release MUST pass between deprecation and removal |
| MUST-BC-03 | All affected product teams MUST acknowledge before a breaking change is merged |
| MUST-BC-04 | Breaking changes MUST be documented in CHANGELOG with migration instructions |
| MUST-BC-05 | Never break a public API silently — even a "minor" behavior change is breaking |

### 13.2 Release Coordination / 发版协调

#### Release Cadence / 发版节奏

- **Patch releases** (bug fixes): 随时可发，无需协调
- **Minor releases** (new features, deprecations): 建议按 **双周** 节奏批量发布，方便产品团队计划
- **Major releases** (breaking changes): 需要**提前 2 周通知**所有产品团队

#### Release Train / 发版列车

```
Week 1: Feature freeze (新功能合并截止)
Week 2: Integration testing (所有产品团队集成测试)
Week 3: Release + publish to crates.io
Week 4: Buffer (产品团队适配、hotfix)
```

#### Cross-Product Integration Testing / 跨产品集成测试

在 minor/major 发布前：

1. **发布 RC (Release Candidate) 版本**到 crates.io（如 `0.2.0-rc.1`）
2. **每个产品团队**在其产品仓库中引用 RC 版本并运行自己的 CI
3. **收集反馈**：任何产品团队发现 breaking 行为变更（即使是 bug fix 导致的），应标记为 blocker
4. **解决所有 blocker** 后，发布正式版本

### 13.3 Conflict Resolution / 冲突解决

#### When Two Products Need Conflicting Changes / 当两个产品需要冲突的变更

```
Product A wants: Add {timeout} param to chat()
Product B wants: Add {retry} param to chat()

Conflict: Both modify same function signature
```

**解决优先级**（由高到低）：

1. **合并需求**：如果两个参数可以共存 → 都添加，函数签名变为 `chat(..., timeout: ..., retry: ...)`
2. **Builder 模式**：如果参数过多 → 重构为 `ChatRequest { timeout, retry, ... }` builder 模式
3. **Feature 分离**：如果需求不可调和 → 通过 feature flag 提供两种实现，产品按需选择
4. **分层设计**：一个放基础 trait，另一个通过 extension trait 添加
5. **Escalate**：团队无法达成共识 → 由 maintainer/core team 决策

#### Emergency Hotfix Protocol / 紧急修复协议

当某个产品发现严重 bug 需要紧急修复时：

1. **创建 hotfix 分支**：从 `main` 分支创建
2. **最小化修复**：只修 bug，不重构，不改 API
3. **加速 review**：在 issue/PR 标注 `HOTFIX`，联系 maintainer 加速 review
4. **合并后立即发布** patch 版本
5. **事后通知**：通知所有产品团队有新的 patch 版本可用

### 13.4 Communication Channels / 沟通渠道

| Channel | Purpose |
|---|---|
| **GitHub Issues** | Bug report, feature request, breaking change notification, cross-team discussion |
| **PR description** | Impact assessment, coordination tracking |
| **CHANGELOG.md** | Release notes, migration guides |
| **Internal team channel** | Urgent notifications, release coordination, hotfix alerts |

### 13.5 Quick Decision Matrix / 快速决策矩阵

| Situation | Action |
|---|---|
| I need a small bug fix | Normal PR, no coordination needed |
| I need a new feature for my product only | Check: does it serve 2+ products? If no → implement in product repo, not here |
| I need a new feature that might serve others | Open issue → discuss → if 2+ products agree → PR with feature flag |
| I need to change a public API | Is it breaking? Yes → deprecation cycle. No → normal PR with review |
| Another team's change broke my product | Open issue immediately, CC maintainer. Hotfix if severe |
| I want to release a new version | Check release cadence. Patch → go ahead. Minor → batch with others. Major → 2 week notice |
| Two teams need incompatible changes | Escalate to maintainer for architectural decision |

### Counter-Examples / 反例

> **Bad**: Product A 直接修改 `LlmProvider` trait 添加 `chat_with_context()` 方法（breaking：所有实现该 trait 的产品都要修改），没有经过 deprecation 流程，Product B 的 CI 直接挂掉。
> **Bad**: Product A directly modifies the `LlmProvider` trait to add `chat_with_context()` method without deprecation. Product B's CI breaks immediately.

> **Bad**: Product A 需要一个新的 search backend，在 `xz-search` 中硬编码了 Writing 产品的特定过滤逻辑（违反了复用性），Product B 使用时出现怪异行为。
> **Bad**: Product A adds a search backend with Writing-specific filtering logic hardcoded. Product B gets unexpected behavior.

> **Bad**: 两个产品团队同时发了 PR 修改同一个 crate，各自独立开发，没有在 issue 中先协调，导致 merge conflict 后需要大量重构。
> **Bad**: Two product teams independently submit PRs modifying the same crate without prior coordination, resulting in large refactors after merge conflicts.
