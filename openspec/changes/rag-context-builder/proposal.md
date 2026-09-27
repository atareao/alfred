# Real RAG in ContextBuilder

## Intent

Replace the hardcoded placeholder RAG memories (`rag_memories: vec!["memory1", "memory2"]`) in `ContextBuilder::build()` with real vector search queries against `vec_memory` using cosine similarity via `MemoryRepo::search_by_vector()`.

The `ContextBuilder` struct gains optional `pool: Option<SqlitePool>` and `provider: Option<Arc<dyn EmbeddingProvider>>` fields so it can be configured after construction without changing the `new()` signature (used in 12+ call sites).

## Scope

### Incluye
1. Add `pool`, `provider`, `rag_budget_tokens` fields to `ContextBuilder` struct
2. Implement real RAG in `build(RAG, profile_id, user_message)`: embed user message → search vec_memory → format memories → include in BuiltContext
3. Fall back to hardcoded memories when pool/provider are not configured
4. Tests: RED → GREEN for all scenarios (with pool, without pool, empty results, budget respected)

### No incluye
- Changes to `ContextBuilder::new()` signature (must remain `fn new() -> Self`)
- Wiring pool/provider in lib.rs / agent.rs / stream.rs (user will do separately)
- Changes to other strategies (SlidingWindow, Historical)

## Impact

### Struct changes
- `ContextBuilder` changes from unit struct to struct with fields
- `new()` still works without args (uses Default)

### No breaking changes
- All existing call sites continue to work unchanged
- `Orchestrator` struct and `new()` unchanged