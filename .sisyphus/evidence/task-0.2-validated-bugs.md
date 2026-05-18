# Task 0.2 — Validated Bug Cross-Reference

**Timestamp**: 2026-05-17T09:41:06Z  
**Build Result**: FAILED (exit 101) — `xz-provider` only crate with compilation errors

## Build Summary

| Crate | Status | Errors | Warnings |
|-------|--------|--------|----------|
| xz-provider | ❌ FAILED | 5 | 3 |
| xz-embed | ✅ PASS | 0 | 5 |
| xz-search | ✅ PASS | 0 | 11 |
| xz-rerank | ✅ PASS | 0 | 5 |
| xz-notification | ✅ PASS | 0 | 1 |
| Others | ⏳ UNKNOWN | blocked | blocked |

**Blocked crates** (depend on xz-provider): xz-agent, xz-memory, xz-rag, xz-skill, xz-knowledge-graph, xz-tts

## Confirmed Compilation Errors

All 5 errors in `xz-provider/src/providers/claude.rs`:

| # | Error Code | Line | Issue | Root Cause |
|---|-----------|------|-------|------------|
| 1 | E0425 | 530 | `cached_tokens` not in scope | Variable defined as `cached` (line 524) but referenced as `cached_tokens` |
| 2 | E0599 | 297 | `.kind()` not found on `reqwest::Error` | reqwest 0.12 removed `kind()` method |
| 3 | E0308 | 345 | Type mismatch in `unwrap_or` | `request.model` is `Option<String>`, not `&str` |
| 4 | E0599 | 439 | `.kind()` not found (duplicate) | Same as #2 |
| 5 | E0004 | 185 | Non-exhaustive match on `ContentPart` | Missing `AudioBase64` and `File` variants |

## Plan Task Status Verification

| Plan Task | Bug | Status | Notes |
|-----------|-----|--------|-------|
| 1.1 | xz-agent action/llm.rs model.map() | ❓ Cannot verify | Blocked by xz-provider not compiling |
| 1.2 | xz-memory provider trait path | ❓ Cannot verify | Blocked by xz-provider not compiling |
| 2.1 | fact_category_to_str data loss | ⚠️ Theory (runtime) | Not a compilation error |
| 2.2 | latency_tracker never updates | ⚠️ Theory (runtime) | Not a compilation error |
| 2.3 | SSE cross-chunk parsing | ⚠️ Theory (runtime) | Not directly visible in compilation |
| 2.4 | xz-embed empty filter SQL | ⚠️ Theory (runtime) | xz-embed compiled successfully |
| 3.1-6.6 | All remaining bugs | ❓ Cannot verify | Blocked by xz-provider |

## Gap Identified

**The plan's Phase 1 does not cover the actual compilation blocker.** The 5 errors in xz-provider/claude.rs are prerequisite fixes that must be addressed before Phase 1 tasks 1.1/1.2 can be verified.

## Action

→ Delegate fix for xz-provider/claude.rs (Task 0.3 — preliminary fix)
→ Re-build to confirm
→ Then proceed with Phase 1 tasks
