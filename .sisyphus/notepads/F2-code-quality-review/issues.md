# F2: Code Quality Review — Issues

## Critical Blockers
1. xz-modules: 2 test failures (`test_sse_broken_per_chunk_lines_demo`, `router_latency_persistence`)
2. Both repos: clippy `-D warnings` is NOT clean (68 + 21 errors)
3. writer: `unsafe` code in `embed/src/storage.rs` violates `forbid(unsafe_code)`

## Pre-existing Issues (not task scope)
- Heavy unwrap/expect usage in both repos (~800+ total across library code)
- Many crates lack tests (0-test crates in writer)
