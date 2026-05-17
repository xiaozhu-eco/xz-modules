# Cross-Document Consistency Check

Checked: 2026-05-17
Documents: AGENTS.md, CONTRIBUTING.md, DEVELOPMENT.md
Compared against: .github/workflows/ci.yml, Cargo.toml

---

## 文档间一致性 [FAIL]

Conflicts found:

1. **CONTRIBUTING.md § Code Style — severity mismatch on fmt/clippy**
   - Line 153: "These are SHOULD rules — strongly recommended but not blocking for CI."
   - Lines 155-156: Lists `cargo fmt` and `cargo clippy` under these SHOULD rules
   - But CI (ci.yml lines 24, 27) **DOES block** on fmt and clippy
   - CONTRIBUTING.md's own § CI Enforcement (lines 200-206) confirms they are CI-enforced
   - **Fix**: Move fmt/clippy out of the "SHOULD" block, or change "not blocking for CI" to "enforced by CI"

2. **CONTRIBUTING.md § Inclusion Criteria — missing test-helper unwrap/expect rule**
   - AGENTS.md line 33: "MUST NOT: Use .unwrap() or .expect() in library test helper code"
   - DEVELOPMENT.md MUST-SE-05: "No .unwrap() or .expect() in library test helper code"
   - CONTRIBUTING.md line 78 only says "library code", never mentions test helper code
   - A contributor reading only CONTRIBUTING.md might think unwrap in test helpers is allowed
   - **Fix**: Add test helper code restriction to CONTRIBUTING.md inclusion criteria and review checklist

3. **CONTRIBUTING.md § Testing — mixed severity under MUST header**
   - Line 164: "These are MUST rules — CI enforces them."
   - Line 168: "Public APIs: **Should** include doc tests" (lowercase) — uses "Should" not "Must"
   - Minor inconsistency but semantically valid (within "MUST" section, one item is SHOULD)
   - **Fix**: Clarify: "Public APIs: Should (recommended, not CI-enforced)"

4. **DEVELOPMENT.md § 8 Testing Standards — lists `cargo build` as MUST, CI has no standalone build**
   - Line 406: "Build: cargo build --workspace --all-features must pass"
   - CI does not run a standalone `cargo build`; build is implicitly tested by `cargo test` and `cargo clippy`
   - Not a real misalignment (build is covered), but the command as written is not in CI
   - **Fix**: Either note that build is implicitly verified, or add a build job to CI

5. **DEVELOPMENT.md § 8 CI Gates — says "four CI jobs", CI has 3**
   - Line 440: "All four CI jobs must pass"
   - CI has 3 jobs: `check` (fmt+clippy combined), `test`, `doc`
   - Documents count fmt and clippy as separate checks (logically correct, structurally inaccurate)
   - **Fix**: Change to "four checks across three CI jobs" or equivalent

## CI 对齐 [PASS]

All 4 CI-enforced checks are documented across all three documents:

| CI Check | ci.yml | AGENTS.md | CONTRIBUTING.md | DEVELOPMENT.md |
|----------|--------|-----------|-----------------|----------------|
| `cargo fmt` | ✅ L24 | ✅ L57 | ✅ L201 | ✅ L442 |
| `cargo clippy` | ✅ L27 | ✅ L59 | ✅ L202 | ✅ L443 |
| `cargo test` | ✅ L38 | ✅ L53 | ✅ L203 | ✅ L444 |
| `cargo doc` | ✅ L47 | ✅ L62 | ✅ L204 | ✅ L445 |

No misalignments in the commands themselves.

## Workspace 对齐 [PASS]

| Constraint | Cargo.toml | AGENTS.md | CONTRIBUTING.md | DEVELOPMENT.md |
|------------|-----------|-----------|-----------------|----------------|
| `unsafe_code = "forbid"` | L73 | L29 | L75 | L140 |
| 11 crate members | L3-15 (11) | L5 "11 crates" | (not stated) | (not stated) |
| License MIT/Apache-2.0 | L22 | (not stated) | L191-194 | (not stated) |
| `edition = "2024"` | L19 | — | — | — |
| `rust-version = "1.85"` | L20 | — | — | — |

**Notes:**
- `edition = "2024"` is not mentioned in any doc — not a conflict, but could be documented
- `rust-version = "1.85"` is not mentioned in any doc — not a conflict, but could be documented (README does mention it)
- All hard workspace constraints (`unsafe_code`, member count) are accurately reflected

## VERDICT: **REJECT**

3 actionable issues found in CONTRIBUTING.md (severity mismatch, missing rule, wording inconsistency) and 2 minor issues in DEVELOPMENT.md. None are blocking for correctness, but the severity mismatch (SHOULD vs CI-enforced) could mislead contributors. All CI commands and workspace constraints are correctly aligned.
