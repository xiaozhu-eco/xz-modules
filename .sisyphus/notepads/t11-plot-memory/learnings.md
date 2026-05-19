# T11: PlotMemory — Learnings

## Implementation Summary

Created `xz-memory/src/domain/plot.rs` with:

- **Types**: `ArcStatus`, `PlotArc`, `ArcProgress`, `VolumeContext`
- **PlotMemory struct**: wraps `Arc<dyn MemorySystem> + novel_id`
- **Methods**:
  - `upsert_arc` — stores arc as Fact with `category=Custom("PlotArc")`, `predicate="plot_arc"`, `subject=arc_id`
  - `get_arc` — retrieves single arc by ID via `recall_facts` + in-RAM filter
  - `get_all_arcs` — retrieves all arcs (empty query + category filter)
  - `get_active_arcs` — filters arcs with `ArcStatus::Active`
  - `get_arc_progress` — calculates completion from key_events vs unresolved_threads ratio
  - `update_after_chapter` — updates arcs containing given chapter (or in chapter range)
  - `get_volume_context` — retrieves VolumeContext stored as Fact with `category=Custom("VolumeContext")`

## Key Design Decisions

1. **Serialization**: Full `PlotArc`/`VolumeContext` serialized to JSON in Fact's `object` field. Fact's own timestamps use epoch millis; domain timestamps use `DateTime<Utc>` and are round-tripped through serde in the JSON blob.

2. **Upsert semantics**: Leverages `MemorySystem::remember_fact` which deduplicates by `(user_id, subject, predicate)`. This means calling `upsert_arc` with the same `arc_id` replaces the previous version.

3. **Progress calculation**: Uses `key_events.len()` as proxy for "resolved threads". Total = key_events + unresolved_threads. This is a heuristic suitable for creative writing workflows.

4. **`update_after_chapter`**: Updates all arcs that have the chapter in `chapters_in_arc` OR whose `start_chapter..end_chapter` range includes it.

## Additions to Other Files

- `xz-memory/Cargo.toml`: Added `chrono = { workspace = true }`
- `xz-memory/src/lib.rs`: Added `pub mod domain;`
- `xz-memory/src/domain/mod.rs`: Added `pub mod plot; pub use plot::PlotMemory;` (shared with parallel tasks)
- `xz-memory/src/domain/character.rs`: Created minimal placeholder (needed because parallel task declared `pub mod character`)

## Parallel Task Conflicts

- Multiple tasks (T9, T10, T11, T12) wrote to `domain/mod.rs` concurrently — the final version includes all modules.
- Other tasks introduced compile errors in `style.rs` (type mismatch) and `Cargo.toml` (duplicate chrono key) — fixed to unblock builds.
