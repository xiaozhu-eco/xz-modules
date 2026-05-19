# xz-modules — Agent Guide

## Project Overview

xz-modules is a Rust workspace providing the core infrastructure crates for the 小竹 (XiaoZhu) AI ecosystem. It contains 11 crates published to [crates.io](https://crates.io), all dual-licensed under MIT OR Apache-2.0.

This repository is **NOT** a product codebase. It serves as a shared foundation — any crate you add or modify here will be depended upon by all 小竹 products. Every change must be evaluated against strict inclusion criteria before proceeding.

## Core Constraints

These are **MUST** and **MUST NOT** rules. Violations must be caught before committing.

### Reusability & Scope
- **MUST**: Only reusable infrastructure capabilities that serve 2+ products belong here
- **MUST NOT**: Add any product-specific business logic, UI components, or workflow code
- **MUST NOT**: Add one-off integrations designed for a single product's unique needs

### API & Interface Stability
- **MUST**: All public APIs must have complete rustdoc documentation
- **MUST**: Use trait abstractions for swappable implementations (facilitates testing)
- **MUST NOT**: Expose internal implementation details in public API surface

### Performance
- **MUST**: All I/O operations must use async (tokio); never block in async contexts
- **MUST NOT**: Use `std::sync::Mutex` or `std::sync::RwLock` in async code (use `tokio::sync`)
- **MUST**: Critical code paths must have benchmarks; no regressions without justification

### Security & Safety
- **MUST**: No `unsafe` code allowed (enforced at workspace level: `forbid(unsafe_code)`)
- **MUST**: No hardcoded secrets, API keys, or tokens in source code
- **MUST**: All public API functions must validate inputs; never panic on malformed data
- **MUST NOT**: Use `.unwrap()` or `.expect()` in library code — propagate errors via `Result`
- **MUST NOT**: Use `.unwrap()` or `.expect()` in library test helper code — use `?` or `.ok()`

### Dependencies
- **MUST**: All dependency versions must be declared in workspace `Cargo.toml` `[workspace.dependencies]`
- **MUST NOT**: Add dependencies with licenses incompatible with MIT or Apache-2.0
- **MUST NOT**: Add unmaintained or abandoned crates
- **MUST**: Minimize new dependency count — prefer stdlib solutions when feasible

### Workspace Dependency Enforcement
- **MUST**: Every crate MUST reference external dependencies via `{ workspace = true }` — NEVER declare a version string in a crate `Cargo.toml` when that dependency already exists in `[workspace.dependencies]`
- **MUST**: Internal workspace crates (e.g. `xz-provider`, `xz-embed`) MUST be referenced via `{ workspace = true }`, not raw `path = "..."`. The workspace root declares both `version` and `path` for each internal crate
- **MUST**: When a crate needs extra features beyond the workspace default, use `{ workspace = true, features = ["extra"] }`
- **MUST**: Every crate's `Cargo.toml` MUST include `[lints] workspace = true` to inherit workspace lint policies (`forbid(unsafe_code)`, etc.)

### Agent Pre-Commit Enforcement (NON-NEGOTIABLE)

Before committing ANY code change, the agent MUST pass ALL of the following gates:

| # | Gate | Command / Check | Rationale |
|---|------|-----------------|-----------|
| 1 | **No library unwrap/expect** | `grep` for `.unwrap()` and `.expect(` in `src/` (not inside `#[cfg(test)]` or `///` doc examples) | Panics in library code crash downstream products |
| 2 | **No std locks in async** | `grep` for `std::sync::Mutex` and `std::sync::RwLock` in `src/` — if found, verify they are NOT inside `async fn` bodies | Causes deadlocks in tokio runtime |
| 3 | **Workspace dep compliance** | Verify new/changed deps use `{ workspace = true }` in crate `Cargo.toml` | Prevents version drift across products |
| 4 | **Clippy clean** | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | CI gate, must pass |
| 5 | **Test green** | `cargo test --workspace --all-features` | CI gate, must pass |
| 6 | **Doc clean** | `cargo doc --workspace --all-features --no-deps` (no broken links or missing docs warnings) | CI gate, must pass |
| 7 | **New public API documented** | Every new `pub fn`, `pub struct`, `pub trait`, `pub enum` MUST have `///` rustdoc | API consumers need docs |

**If ANY gate fails → fix before committing. No exceptions.**

### Concurrent Multi-Product Awareness

This repo is shared by ALL 小竹 products. When modifying any crate:

- **MUST**: Assess whether the change is breaking (API signature change, behavior change, trait bound addition) — see [DEVELOPMENT.md §3](./DEVELOPMENT.md#3-api-design--api-设计)
- **MUST**: For breaking changes: follow the deprecation cycle (deprecate in current minor, remove in next major)
- **MUST**: Before adding a new feature, verify it serves 2+ products (see inclusion criteria above)
- **MUST**: If a product needs a one-off extension, use trait extension or feature-gating — never modify a shared trait for a single consumer
- **MUST**: Check with other product teams before merging changes that affect public API surface — use the PR as the coordination point

## Quick Reference

- [CONTRIBUTING.md](./CONTRIBUTING.md) — Open-source contribution guide (PR process, code style, testing)
- [DEVELOPMENT.md](./DEVELOPMENT.md) — Detailed development specification (5-dimension criteria, error handling, async patterns, versioning)

## Development Commands

```bash
# Build all crates
cargo build --workspace --all-features

# Run all tests
cargo test --workspace --all-features

# Check formatting
cargo fmt --all -- --check

# Run lints
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Generate documentation
cargo doc --workspace --all-features --no-deps
```

## CI Enforcement

The CI pipeline (`.github/workflows/ci.yml`) enforces:
1. `cargo fmt --all -- --check` — formatting
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings` — linting
3. `cargo test --workspace --all-features` — testing
4. `cargo doc --workspace --all-features --no-deps` — documentation

All four must pass before any PR can be merged.
