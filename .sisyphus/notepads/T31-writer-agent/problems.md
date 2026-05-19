# Issues & Problems

## Build blocked by pre-existing errors
- writer crate has 19 compilation errors in `post_processing.rs` (missing `volumn_outline` crate references, type inference issues)
- These are NOT caused by T31 changes
- `agent.rs` compiles clean when checked individually (`cargo check` reports 0 agent-related errors)
- `cargo test -p writer -- workflow::agent` also fails due to these pre-existing lib errors
- Resolution: these pre-existing errors need a separate fix before agent tests can run end-to-end

## LSP unavailable
- `rust-analyzer` not installed in current toolchain
- Cannot run `lsp_diagnostics` for IDE-level verification
- Relying on `cargo check` output instead
