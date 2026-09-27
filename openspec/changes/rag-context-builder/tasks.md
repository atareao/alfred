# Tasks: Real RAG in ContextBuilder

## TDD Task Checklist

### Fase 1: Struct changes (no tests needed — pure refactor)
- [ ] 1.1 Agregar campos `pool`, `provider`, `rag_budget_tokens` a `ContextBuilder`
- [ ] 1.2 Implementar `Default` trait
- [ ] 1.3 Mantener `fn new() -> Self` (misma firma)

### Fase 2: RED — Tests for RAG ContextBuilder
- [ ] 2.1 RED: `test_rag_with_pool_and_provider` — Configurar builder con pool + mock provider, verificar que consulta vec_memory
- [ ] 2.2 RED: `test_rag_without_pool` — Sin pool, cae a hardcoded
- [ ] 2.3 RED: `test_rag_empty_results` — vec_memory vacía → rag_memories vacío
- [ ] 2.4 RED: `test_rag_budget_respected` — Memorias con tokens que exceden budget → no se incluyen
- [ ] 2.5 RED: `test_sliding_window_no_rag` — SlidingWindow no llama a vec_memory
- [ ] 2.6 RED: `test_historical_no_rag` — Historical no llama a vec_memory

### Fase 3: GREEN — Implement real RAG
- [ ] 3.1 GREEN: Implementar `build(RAG, ...)` con flujo pool+provider → embed → search_by_vector → format
- [ ] 3.2 GREEN: Implementar fallback a hardcoded cuando pool/provider son None
- [ ] 3.3 GREEN: Asegurar que token_estimate incluye tokens de memorias
- [ ] 3.4 GREEN: Ejecutar tests: `cargo test --lib orchestrator::context_builder -- --test-threads=1`

### Fase 4: REFACTOR
- [ ] 4.1 Ejecutar `cargo clippy --lib -- -D warnings`
- [ ] 4.2 Ejecutar `cargo fmt --check`
- [ ] 4.3 Verificar que todos los tests pasan