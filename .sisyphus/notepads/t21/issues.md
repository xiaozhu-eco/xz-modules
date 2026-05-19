# T21 Issues

## Pre-existing Build Issues

1. **`libsqlite3-sys` version conflict**: `xz-knowledge-graph` (sqlx 0.7 → libsqlite3-sys 0.26) vs `embed` (sqlx 0.8 → libsqlite3-sys 0.30). Fixed by updating `xz-knowledge-graph`, `xz-memory`, `xz-embed`, `xz-skill` to sqlx 0.8.

2. **`writer-client/src-tauri` dependency resolution**: Depends on `xz-sdk` → `xz-provider` (git) → `xz-auth-client` (git, not found). Temporarily commented out from workspace members to build awe-tools. Pre-existing issue.

3. **`provider/src/adapter.rs` type mismatch**: `def.api_key = api_key` where `api_key` is `String` but field is `Option<String>`. Fixed with `Some(api_key)`.

4. **Pre-existing compilation errors in query tools**: `recent_history.rs` (PageRequest limit type mismatch u32→usize), `style_profile.rs` (borrow after move), `world_rules.rs` (unused import). Fixed all.

## Module Registration

The `query/mod.rs` must declare modules and re-export structs for new tools. Initially only had `characters`, updated to include all 9 query tools.
