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
