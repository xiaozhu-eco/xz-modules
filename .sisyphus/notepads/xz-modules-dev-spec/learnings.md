# Learnings — xz-modules-dev-spec

## Conventions
- AGENTS.md: pure English, ≤200 lines, structured as MUST/MUST NOT rules
- CONTRIBUTING.md: bilingual (EN + CN), MUST for core rules, SHOULD for style
- DEVELOPMENT.md: bilingual (EN + CN), all MUST, 5-dimension criteria with counter-examples
- Spec describes ideal target state; existing bugs tracked separately in audit plan
- Rust 2024 edition, MSRV 1.85, `unsafe_code = "forbid"`
- Workspace-level dependency management via [workspace.dependencies]

## Gotchas
- thiserror versions are inconsistent across crates (workspace=2, some crates=1.0)
- edition is inconsistent (workspace=2024, some crates=2021)
- xz-notification has its own CI workflow anomaly
- No rustfmt.toml, clippy.toml, or rust-toolchain.toml exist yet
- CI enforces: fmt, clippy -D warnings, test, doc

## Decisions
- Language: AGENTS.md English-only, CONTRIBUTING.md + DEVELOPMENT.md bilingual
- Governance model: deferred (not in scope)
- Tone: CONTRIBUTING.md layered MUST/SHOULD; DEVELOPMENT.md all MUST
