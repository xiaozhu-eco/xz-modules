# Changelog

## [0.2.0] - 2026-05-20

### Added
- `#![warn(missing_docs)]` lint for public API documentation enforcement
- `[lints] workspace = true` for consistent workspace linting
- `is_retryable()` method on `SkillError`
- Crate-level doc comments describing crate purpose and feature flags
- CHANGELOG.md for tracking version changes

### Changed
- Dependencies changed from inline versions to `{ workspace = true }` references
  (`serde`, `serde_json`, `serde_yaml`, `async-trait`, `uuid`)
- Version bumped from 0.1.0 to 0.2.0
