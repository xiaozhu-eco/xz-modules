# Contributing to xz-modules

> xz-modules is the infrastructure foundation of the 小竹 AI ecosystem. Every crate published here is depended upon by all 小竹 products. Quality and discipline are non-negotiable.
>
> xz-modules 是小竹 AI 生态的基础设施仓库。这里发布的每个 crate 都会被所有小竹产品依赖。质量和纪律不容妥协。

Also see:
- [AGENTS.md](./AGENTS.md) — AI agent core constraints and quick reference
- [DEVELOPMENT.md](./DEVELOPMENT.md) — Detailed development specification

---

## Table of Contents / 目录

- [How to Contribute / 如何贡献](#how-to-contribute--如何贡献)
- [Inclusion Criteria / 纳入标准](#inclusion-criteria--纳入标准)
- [PR Process / PR 流程](#pr-process--pr-流程)
- [Commit Convention / 提交规范](#commit-convention--提交规范)
- [Code Style / 代码风格](#code-style--代码风格)
- [Testing / 测试](#testing--测试)
- [Review Checklist / 审查清单](#review-checklist--审查清单)
- [License / 许可证](#license--许可证)

---

## How to Contribute / 如何贡献

### Reporting Bugs / 报告 Bug

- Search existing [issues](https://github.com/xiaozhu-eco/xz-modules/issues) first
- Use the bug report template when creating a new issue
- Include: Rust version, OS, minimal reproduction code, expected vs actual behavior
- Label the crate affected (e.g., `xz-provider`, `xz-embed`)

### Suggesting Features / 建议功能

- Open a discussion first — not all features belong in xz-modules (see [Inclusion Criteria](#inclusion-criteria--纳入标准))
- Explain: use case, which 小竹 products benefit, proposed API sketch
- Wait for maintainer feedback before starting implementation

### Code Contributions / 代码贡献

1. Ensure your change meets the [Inclusion Criteria](#inclusion-criteria--纳入标准)
2. Follow the [PR Process](#pr-process--pr-流程)
3. All [CI checks](#ci-enforcement) must pass
4. All [Review Checklist](#review-checklist--审查清单) items must be addressed

---

## Inclusion Criteria / 纳入标准

**Only reusable infrastructure capabilities belong in xz-modules.** Every proposed change must satisfy ALL applicable criteria below.

**只有可复用的基础能力才应加入 xz-modules。** 每项变更必须满足以下所有适用标准。

### MUST: Reusability / 复用性

- Can this capability be used by **2+ products** without modification?
- Is it solving a **domain-independent** problem (LLM calls, embeddings, search, scheduling), not a product-specific workflow?

### MUST: Interface & Stability / 接口与稳定性

- Does it expose a **stable public API** with complete rustdoc?
- Does it use **trait abstractions** for swappable implementations?
- Is the API **free of UI/business coupling**?

### MUST: Performance / 性能

- Are all I/O operations **async** (tokio)?
- Are `std::sync::Mutex`/`RwLock` avoided in async contexts? (Use `tokio::sync`)
- Are critical paths benchmarked, with no regression without justification?

### MUST: Security & Safety / 安全

- No `unsafe` code allowed (workspace-level `forbid(unsafe_code)`)
- No hardcoded secrets, API keys, or tokens
- All public functions validate inputs; never panic on malformed data
- No `.unwrap()` or `.expect()` in library code — propagate errors via `Result`
- No `.unwrap()` or `.expect()` in library test helper code — use `?` or `.ok()`

### MUST: Dependencies / 依赖

- License compatible with MIT OR Apache-2.0
- The dependency is actively maintained (not abandoned)
- Minimize new dependency count; prefer stdlib solutions
- Version declared in workspace `Cargo.toml` `[workspace.dependencies]`

### Exclusions / 排除项

The following **MUST NOT** be added to xz-modules:

- Product-specific workflows or business logic
- UI components, templates, or user interaction code
- Business rules (pricing, feature flags, product-specific validation)
- Product-specific configuration (API keys, endpoints, settings)
- One-off integrations for a single product's unique needs

### Decision Flow / 判断流程

```
Proposed change
  ├─ Is it cross-product (2+ products)?
  │  └─ NO → REJECT (product-specific)
  ├─ Is it domain-independent (generic, not business logic)?
  │  └─ NO → REJECT (business-specific)
  ├─ Does it expose a stable, documented API?
  │  └─ NO → REJECT (poor abstraction)
  ├─ Is it async-first, safe, and minimal-dependency?
  │  └─ NO → REJECT (fails quality gate)
  └─ ALL YES → ACCEPT
```

---

## PR Process / PR 流程

1. **Fork** the repository
2. **Branch**: Create a feature branch from `main`:
   - `fix/short-description` for bug fixes
   - `feat/short-description` for new features
   - `docs/short-description` for documentation changes
3. **Commit**: Follow [Commit Convention](#commit-convention--提交规范)
4. **PR**: Open a pull request against `main`
5. **CI**: All checks must pass (fmt, clippy, test, doc)
6. **Review**: At least one maintainer must approve
7. **Merge**: Squash-merge to maintain a clean history

---

## Commit Convention / 提交规范

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `perf`
Scope: crate name (e.g., `xz-provider`, `xz-memory`)

Examples:
- `feat(xz-provider): add OpenAI streaming support`
- `fix(xz-embed): correct cosine similarity normalization`
- `docs: update README with architecture diagram`

---

## Code Style / 代码风格

These are standards adopted by the project. `cargo fmt` and `cargo clippy` are **CI-enforced** (see [CI Enforcement](#ci-enforcement)). Naming and structure conventions below are strongly recommended.

本项目采用以下代码风格标准。`cargo fmt` 和 `cargo clippy` 由 **CI 强制执行**。命名和结构规范为强烈建议。

- Run `cargo fmt --all -- --check` before committing
- Follow `cargo clippy --workspace --all-targets --all-features -- -D warnings` guidance
- Naming: `snake_case` for functions/variables, `CamelCase` for types/traits, `SCREAMING_SNAKE_CASE` for constants
- Module structure: one module per file, re-exported via `pub use` in `lib.rs`

---

## Testing / 测试

These are **MUST** rules — CI enforces them.

- **New features**: Must include unit tests covering: happy path, error cases, edge cases
- **Bug fixes**: Must include a regression test that fails before the fix and passes after
- **Public APIs**: Should include doc tests (`/// ``` ... ``` `) demonstrating usage
- Run all tests: `cargo test --workspace --all-features`

---

## Review Checklist / 审查清单

Code reviewers **MUST** check:

- [ ] Does the change meet [Inclusion Criteria](#inclusion-criteria--纳入标准)?
- [ ] Is the public API documented with rustdoc?
- [ ] Are there no `.unwrap()` or `.expect()` calls in library code?
- [ ] Are there no hardcoded secrets or API keys?
- [ ] Are there no `std::sync::Mutex`/`RwLock` in async contexts?
- [ ] Are new dependencies declared in workspace `Cargo.toml`?
- [ ] Are new dependencies compatible with MIT/Apache-2.0 licenses?
- [ ] Do tests cover: happy path, error cases, edge cases?
- [ ] Do all CI checks pass?

---

## License / 许可证

By contributing, you agree that your contributions will be licensed under:

- [MIT License](LICENSE-MIT)
- [Apache License 2.0](LICENSE-APACHE)

---

## CI Enforcement

The CI pipeline enforces:
1. `cargo fmt --all -- --check` — formatting
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings` — linting
3. `cargo test --workspace --all-features` — testing
4. `cargo doc --workspace --all-features --no-deps` — documentation

All four must pass before merging.
