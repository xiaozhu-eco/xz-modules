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
- [Concurrent Modification Protocol / 并发修改协议](#concurrent-modification-protocol--并发修改协议)
- [Commit Convention / 提交规范](#commit-convention--提交规范)
- [Code Style / 代码风格](#code-style--代码风格)
- [Testing / 测试](#testing--测试)
- [Review Checklist / 审查清单](#review-checklist--审查清单)
- [Release Coordination / 发版协调](#release-coordination--发版协调)
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

### Prerequisites / 前置条件

Before opening a PR, complete an **impact assessment** (影响评估):

1. Which crates are affected?
2. Is the change Non-breaking or Breaking? (see [API Design §Stability](./DEVELOPMENT.md#3-api-design--api-设计))
3. Which 小竹 products depend on these crates? (list all)
4. Does this change require coordination with other product teams?

> Include the impact assessment in the PR description. See [Concurrent Modification Protocol](#concurrent-modification-protocol--并发修改协议) for details.

### PR Template / PR 模板

```markdown
## Impact Assessment / 影响评估
- **Crates affected**: <list>
- **Change type**: Non-breaking / Breaking
- **Products affected**: <list>
- **Product release timelines**: <if relevant>

## Description / 描述
<what and why>

## Breaking Change Detail / 破坏性变更详情 (if applicable)
- **Deprecation plan**: <#[deprecated] annotation, migration guide>
- **Scheduled removal**: <major version>
- **Product migration status**: <which products have migrated>

## Checklist
- [ ] Inclusion criteria met
- [ ] Public API documented (rustdoc)
- [ ] No unwrap/expect in library code
- [ ] No std::sync locks in async code
- [ ] Dependencies use { workspace = true }
- [ ] Tests cover: happy path, error cases, edge cases
- [ ] Clippy clean, tests green, docs build
- [ ] Affected product teams notified (if breaking)
```

### Steps / 步骤

1. **Fork** the repository
2. **Branch**: Create a feature branch from `main`:
   - `fix/short-description` for bug fixes
   - `feat/short-description` for new features
   - `docs/short-description` for documentation changes
3. **Commit**: Follow [Commit Convention](#commit-convention--提交规范)
4. **PR**: Open a pull request against `main` with the impact assessment
5. **CI**: All checks must pass (fmt, clippy, test, doc)
6. **Review**: At least one maintainer must approve
   - For **breaking changes**: ALL affected product teams must acknowledge before merge
   - For **multi-crate changes**: at least one other product team must approve
7. **Merge**: Squash-merge to maintain a clean history

---

## Concurrent Modification Protocol / 并发修改协议

> 当多个产品团队同时需要修改 xz-modules 时，必须遵守以下协议。
> When multiple product teams need to modify xz-modules concurrently, this protocol MUST be followed.

### Coordination Matrix / 协调矩阵

| Change Type | Review Required | Notification | Deprecation |
|---|---|---|---|
| Bug fix (no API change) | 1 maintainer | None | N/A |
| New feature behind feature flag (default off) | 1 maintainer | Inform other teams | N/A |
| New feature (on by default) | 1 maintainer + 1 product team | Announce in issue | N/A |
| Non-breaking API addition | 1 maintainer + 1 product team | Announce in issue | N/A |
| **Breaking change** | **1 maintainer + ALL affected product teams** | **2+ weeks advance notice** | **#[deprecated] for ≥1 minor release** |
| New crate | 1 maintainer + confirmation 2+ products will use it | Announce in issue | N/A |

### Before You Start / 开始前

1. **Search existing issues/PRs** — is someone already working on the same crate?
2. **Open a discussion/issue first** — describe your proposed change, tag affected product teams
3. **Wait for feedback** — don't start implementation until the approach is agreed upon
4. **For breaking changes**: follow the [deprecation cycle](./DEVELOPMENT.md#13-concurrent-multi-product-development--多产品并发开发规范)

### When Two PRs Conflict / 当两个 PR 冲突

If two product teams open PRs touching the same crate:

1. **Detect early**: PR authors and maintainer identify the conflict
2. **Coordinate**: Discuss in the shared issue — can both changes coexist?
3. **Resolve**:
   - **Compatible** → merge one, rebase the other
   - **Incompatible** → escalate to maintainer for architectural decision (see [Conflict Resolution](./DEVELOPMENT.md#133-conflict-resolution--冲突解决))
4. **Merge order**: Non-breaking changes merge first. Breaking changes merge last (or wait for next major release)

### Emergency Fix Protocol / 紧急修复

For critical bugs affecting production products:

1. Create branch: `hotfix/short-description`
2. Minimize scope: bug fix only, no refactoring, no API changes
3. Tag PR as `HOTFIX` and notify maintainer
4. After merge → immediate patch release
5. Post-release: notify all product teams of the new patch version

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

### Code Review / 代码审查

Code reviewers **MUST** check:

- [ ] Does the change meet [Inclusion Criteria](#inclusion-criteria--纳入标准)?
- [ ] Is the public API documented with rustdoc?
- [ ] Are there no `.unwrap()` or `.expect()` calls in library code (outside `#[cfg(test)]`)?
- [ ] Are there no hardcoded secrets or API keys?
- [ ] Are there no `std::sync::Mutex`/`RwLock` in async contexts?
- [ ] Are new dependencies declared in workspace `Cargo.toml` `[workspace.dependencies]`?
- [ ] Do crate `Cargo.toml` files use `{ workspace = true }` for all deps (no inline versions)?
- [ ] Does the crate use `[lints] workspace = true`?
- [ ] Are new dependencies compatible with MIT/Apache-2.0 licenses?
- [ ] Do tests cover: happy path, error cases, edge cases?
- [ ] Do all CI checks pass? (fmt, clippy, test, doc)

### Cross-Product Review / 跨产品审查

For changes affecting public API:

- [ ] Is the impact assessment complete in the PR description?
- [ ] Have all affected product teams been notified?
- [ ] For breaking changes: has the deprecation cycle been followed? (see [DEVELOPMENT.md §13](./DEVELOPMENT.md#13-concurrent-multi-product-development--多产品并发开发规范))
- [ ] For breaking changes: have ALL affected product teams acknowledged?
- [ ] Does the new API follow existing conventions (naming, error types, async patterns)?
- [ ] Is the feature behind a feature flag if it may only be used by one product?
- [ ] Is the `CHANGELOG.md` updated with migration guidance (if breaking)?

---

## License / 许可证

By contributing, you agree that your contributions will be licensed under:

- [MIT License](LICENSE-MIT)
- [Apache License 2.0](LICENSE-APACHE)

---

## Release Coordination / 发版协调

### Release Cadence / 发版节奏

| Type | Frequency | Coordination Required |
|---|---|---|
| **Patch** (bug fixes) | As needed | None — immediate release |
| **Minor** (features, deprecations) | Bi-weekly train | Notify 1 week in advance |
| **Major** (breaking changes) | As needed (rare) | **2 weeks advance notice** to ALL product teams |

### Minor Release Process / Minor 发版流程

1. **Feature freeze**: Friday — no new features merged after
2. **RC (Release Candidate)**: Published as `x.y.z-rc.1` on Monday
3. **Integration testing**: Product teams test with RC, report blockers by Wednesday
4. **Resolve blockers**: Fix any issues reported
5. **Release**: Friday — publish final version to crates.io
6. **Announce**: Notify all product teams that the new version is available

### Patch Release Process / Patch 发版流程

1. Merge bug fix PR
2. Bump patch version in affected crate(s)
3. Update `CHANGELOG.md`
4. Tag `v<version>` and push
5. CI auto-publishes to crates.io

### Post-Release / 发版后

- Update product repositories to use the new version (each product team responsible for their own upgrade)
- Monitor for regressions for 48 hours after release
- If a regression is found → open `HOTFIX` issue immediately

---

## CI Enforcement

The CI pipeline enforces:
1. `cargo fmt --all -- --check` — formatting
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings` — linting
3. `cargo test --workspace --all-features` — testing
4. `cargo doc --workspace --all-features --no-deps` — documentation

All four must pass before merging.
