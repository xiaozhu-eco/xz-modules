# Wave 3 - CharacterMemory Learnings

## Conventions
- `domain/mod.rs` already existed with plot/seed/style modules; added `character` module to it
- domain modules follow a consistent pattern: `XxxMemory` struct wraps `Arc<dyn MemorySystem>` + `novel_id`, uses `FactCategory::Custom("Xxx")` (or specific enum variant) for storage
- Fact ID format: `{novel_id}:{entity_id}` to avoid collisions across novels (InMemoryMemory uses Fact.id as HashMap key)
- Predicate format: `{type}:{entity_id}` for exact match retrieval
- Subject is a fixed string (`"character"`) for filtering

## Gotchas
1. **Fact ID collisions**: InMemoryMemory stores facts in HashMap<Fact.id, Fact>. Two different novels with the same character_id would overwrite each other. Solution: use `{novel_id}:{character_id}` as Fact.id.
2. **recently_active filtering**: Uses `c.last_appearance >= chapters_ago` — requires last_appearance to be not None and >= the threshold
3. **update_after_chapter**: Only transitions "active" → "dormant"; "introduced" characters who don't appear remain "introduced"

## Patterns
- `character_to_fact()` / `fact_to_character()` conversion helpers (mirrors seed.rs)
- `apply_character_query()` for in-memory filtering (mirrors seed.rs)
- Test helpers: `make_character()` for creating test data
- Tests use `InMemoryMemory` wrapped in `Arc` for testing
