# F2: Code Quality Review — Learnings

## xz-modules workspace
- 68 clippy errors: mostly collapsed if-statements (16), unused imports/variables, clippy style nits
- 2 test failures: SSE buffer test and router latency persistence
- ~450 unwrap/expect calls in library code
- unsafe: clean (only a test that validates forbid(unsafe_code))

## writer workspace  
- 21 clippy errors: mostly unused Result (9), dead code in provider
- All tests pass
- ~363 unwrap/expect calls in library code
- unsafe: `embed/src/storage.rs` has `register_vec_extension` with raw C FFI
