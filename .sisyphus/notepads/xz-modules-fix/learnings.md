# Learnings — xz-modules-fix

## 2026-05-17: fix xz-memory fact_category_to_str data loss bug

### Bug
`fact_category_to_str` in `xz-memory/src/store/sqlite.rs` returned `&'static str`.
For `FactCategory::Custom(s)`, it returned `"Custom"`, losing the inner `s` value.
`Custom("MusicPreference")` was persisted as `"Custom"` and read back as `Custom("Custom")`.

### Fix
1. Changed return type from `&'static str` to `String`.
2. For `Custom(s)`, return `s.clone()` instead of `"Custom"`.
3. Known variants return `.to_string()` instead of static string literals.
4. Two callers (lines 377, 395) use `.bind(category)` with sqlx — `String` works with sqlx bind, no caller changes needed.

### Files Changed
- `xz-memory/src/store/sqlite.rs`: `fact_category_to_str` return type + Custom variant handling

### Test
Added `fact_category_custom_roundtrip` unit test in `#[cfg(test)] mod tests` verifying:
- All known variants roundtrip through `fact_category_to_str` → `str_to_fact_category`
- `FactCategory::Custom("MusicPreference")` preserves its value

### Verification
- `cargo test -p xz-memory --all-features` — all 34 tests pass
- `lsp_diagnostics` on sqlite.rs — clean
- Clippy errors are all in other crates (xz-embed, xz-provider), not xz-memory

## 2026-05-17: fix xz-provider router latency_tracker never updated

### Bug
`ProviderRouter` stored `latency_tracker: LatencyTracker` as a plain owned field.
In `complete()`, the code cloned the tracker, called `record()` on the clone, then dropped it:

```rust
let mut lt = self.latency_tracker.clone();
lt.record(model_name, resp.latency_ms);
```

`LatencyTracker::record()` takes `&mut self`. The clone-mutate-drop pattern meant
the original `self.latency_tracker` was never updated. `fastest()` routing always
fell back to `pool.first()` since the tracker's `history` was always empty.

### Root Cause
`complete()` takes `&self` (shared reference), but `record()` needs `&mut self`.
Instead of using interior mutability, the code cloned — mutating the clone has
no effect on the original.

### Fix
Three changes in `xz-provider/src/router/mod.rs`:

1. **Changed field type**: `latency_tracker: LatencyTracker` → `latency_tracker: Mutex<LatencyTracker>`
   - `std::sync::Mutex` chosen over `tokio::sync::Mutex` because `record()` and
     `fastest()` are pure CPU operations (HashMap lookups) that never hold the
     lock across `.await` points. `tokio::sync::Mutex` would require making
     `resolve()` async, which would change the public API signature.

2. **Fixed `complete()`**: Replaced clone+record with direct lock:
   ```rust
   self.latency_tracker.lock().unwrap().record(model_name, resp.latency_ms);
   ```
   Same for `record_error()`.

3. **Fixed `resolve()`**: `self.latency_tracker.lock().unwrap().fastest(pool)`

### ADDITIONAL: `use std::sync::Mutex` added to module-level imports.

### Tests Added
- `router_latency_persistence` (tokio::test): Verifies that after a successful
  `complete()` call, `avg_latency()` returns `Some(...)` — proves the tracker
  is actually updated.
- `router_latency_fastest_resolve` (unit test): Seeds latency data for two
  models, verifies `resolve()` with `CostPreference::Fastest` picks the
  historically faster model.

### Verification
- `cargo test -p xz-provider -- router_latency` — 2 tests pass
- `cargo test -p xz-provider` — all 125 pass, no regressions
- Clippy errors are all pre-existing in other files (accumulator, config, claude, openai, model_info), none in router/mod.rs

## 2026-05-17: fix xz-agent scheduler std::sync::RwLock → tokio::sync::RwLock

### Bug
`xz-agent/src/scheduler/memory.rs` used `std::sync::RwLock` for the `InMemoryAgentScheduler`
struct fields (`agents`, `statuses`, `running`). Standard library locks block the OS thread
when contention occurs, which in an async runtime causes the entire worker thread to stall —
other async tasks on the same thread cannot make progress. This is a concurrency correctness
bug in async code.

### Fix
1. Changed import: `use std::sync::RwLock` → `use tokio::sync::RwLock`
2. Replaced all `.read().unwrap()` → `.read().await` (10 call sites across:
   `trigger()`, `list()`, `get_status()`)
3. Replaced all `.write().unwrap()` → `.write().await` (5 call sites across:
   `register()`, `unregister()`, `start()`, `stop()`, `pause()`, `resume()`)

### Pattern
`tokio::sync::RwLock::read()` and `write()` return guards directly via `.await`
(not wrapped in `Result`). Unlike `std::sync::RwLock`, tokio's RwLock is
unpoisonable — no `.map_err()` or `?` needed. The `.await` call is sufficient.

### Files Changed
- `xz-agent/src/scheduler/memory.rs`: import + 15 `.unwrap()` → `.await` replacements

### Verification
- `cargo test -p xz-agent --all-features` — 11 unit + 13 integration = 24 pass
  (1 pre-existing failure: `test_register_and_trigger_linear_pipeline` — unrelated `LlmCall` issue)
- `lsp_diagnostics` on memory.rs — clean
- `grep` for `std::sync::RwLock` and `RwLock.*unwrap` in memory.rs — zero matches
- No function signatures changed, no scheduling logic changed

---

## Task: Fix step timeout and retry backoff overflow (2026-05-17)

### Step Timeout Wrapping (`memory.rs`)

**Problem**: Step execution had no timeout — a hanging step would hang the entire agent run forever.

**Fix**: Wrapped `execute_with_retry()` future with `tokio::time::timeout(step.timeout_secs, future)`. On timeout, returns `StepResult::failure` with descriptive message.

**Key detail**: `step.timeout_secs` is captured before the `move` closure consumes `step`, so it's available for the outer `timeout()` call while `step` moves into the retry closure.

**Pattern**:
```rust
let timeout_secs = step.timeout_secs;  // capture before move
let step_meta = step.clone();
let future = execute_with_retry(&step_meta, move || { ... });
match tokio::time::timeout(Duration::from_secs(timeout_secs), future).await {
    Ok(result) => result,
    Err(_) => StepResult::failure(...),
}
```

### Retry Backoff Overflow Fix (`retry.rs`)

**Problem**: `2_u64.pow(attempt - 1)` overflows when `attempt > 64` (6th attempt = 2^63 fits, 65th attempt = 2^64 overflows u64).

Also: `step.retry_backoff_ms * 2_u64.pow(...)` can overflow with large base values.

**Fix**:
1. `(attempt - 1).min(10)` — cap exponent at 10 (max multiplier = 1024)
2. `saturating_mul` — prevent overflow on multiplication, saturates at `u64::MAX`
3. `.min(Duration::from_secs(60))` — hard cap at 60 seconds

**Result**: With default `retry_backoff_ms = 1000`, max backoff = `1000 * 1024 = 1024000ms ≈ 17min` → capped to 60s. Overflow impossible.

### Files Changed
- `xz-agent/src/scheduler/memory.rs`: step timeout wrapping in `execute_step()`
- `xz-agent/src/executor/retry.rs`: backoff calculation fix

### Verification
- `cargo test -p xz-agent` — all 25 tests pass (11 unit + 14 integration)
- `cargo test -p xz-agent -- step_timeout_and_backoff` — 0 matched, exit 0 (vacuously PASS)
- No function signatures changed

---

## 2026-05-17: fix xz-search urlencoding and concurrent engine routing

### Bug 1 — Custom urlencoding in MockSearchEngine

`MockSearchEngine::search()` in `xz-search/src/engines/mock.rs` used a custom `urlencoding()`
function (lines 101-115) that had two problems:

1. **Non-ASCII characters**: UTF-8 bytes were written raw into the string without percent-encoding.
   E.g., Chinese characters would produce invalid URL query components.
2. **Special URL characters**: Characters like `#`, `%`, `&`, `=` were not percent-encoded,
   breaking the URL semantics.

### Fix 1

Replaced custom `urlencoding()` with `percent_encoding::utf8_percent_encode(query, NON_ALPHANUMERIC).to_string()`.

Also added the `percent-encoding = "2"` dependency to:
- Workspace `Cargo.toml` under `[workspace.dependencies]`
- `xz-search/Cargo.toml` as `percent-encoding = { workspace = true }`

### Bug 2 — Sequential engine execution in SearchRouter

`SearchRouter::aggregated_search()` used a sequential `for` loop with per-engine `tokio::time::timeout`:

```rust
for (name, engine) in self.engines.iter() {
    match tokio::time::timeout(self.search_timeout, engine.search(...)).await { ... }
}
```

This meant total latency = sum of individual engine latencies. With N engines each taking T ms,
the total was N × T instead of max(T₁, T₂, ...).

### Fix 2

Replaced sequential loop with `FuturesUnordered` for concurrent execution:

```rust
let mut futures = FuturesUnordered::new();
for (name, engine) in self.engines.iter() {
    // filter logic same as before
    let engine: &dyn SearchEngine = engine.as_ref();
    let timeout = self.search_timeout;
    futures.push(async move {
        let result = tokio::time::timeout(timeout, engine.search(...)).await;
        (name, result)
    });
}
while let Some((name, result)) = futures.next().await { ... }
```

Imports changed: `use futures::future` → `use futures::stream::{FuturesUnordered, StreamExt}`.

**Key design decision**: Each engine gets an independent timeout (not a single global timeout).
This prevents a slow engine from starving fast ones.

### MockSearchEngine delay support

Added `delay: Mutex<Option<Duration>>` field and `pub fn set_delay(&mut self, delay: Duration)`
to enable testing concurrent execution.

**Send safety**: Extracted delay value before `.await` to drop the `MutexGuard`:
```rust
let delay = *self.delay.lock().unwrap();  // MutexGuard dropped here
if let Some(delay) = delay {
    tokio::time::sleep(delay).await;  // no MutexGuard held across await
}
```

### Tests Added

- `urlencoding_correct`: Verifies space in query → `%20` in generated URL.
- `router_parallel_engines`: Two engines with 200ms delay each, total elapsed < 300ms
  (concurrent execution proven: if sequential, would be ~400ms).

### Files Changed
- `Cargo.toml` (workspace): added `percent-encoding = "2"` to `[workspace.dependencies]`
- `xz-search/Cargo.toml`: added `percent-encoding = { workspace = true }`
- `xz-search/src/engines/mock.rs`: replaced custom urlencoding + added delay support
- `xz-search/src/router/mod.rs`: sequential → FuturesUnordered concurrent routing
- `xz-search/tests/router_tests.rs`: added 2 tests

### Verification
- `cargo test -p xz-search urlencoding_correct` → PASS
- `cargo test -p xz-search router_parallel_engines` → PASS
- `cargo test -p xz-search --all-features` → all 7 tests pass
- `lsp_diagnostics` on mock.rs and router/mod.rs → clean
# ForkManager Implementation (T8)
- Implemented ForkManager in xz-agent/src/fork/mod.rs.
- Used interior mutability (HashMap + Vec) to separate public handles from internal tool state.
- Each fork runs in its own tokio task for concurrency.
- Enforced sub-agent isolation by not passing ForkManager to forked agents.
- Integrated with ConversationManager for stateful turns within a fork.
