# TDD Task Checklist — SQL Window History

## RED (Write failing tests)

- [ ] **RED-1**: Test `list_by_token_budget` selecciona mensajes dentro del presupuesto
- [ ] **RED-2**: Test `list_by_token_budget` con presupuesto suficiente para todos
- [ ] **RED-3**: Test `list_by_token_budget` usa `collapsed_tokens_count` cuando existe
- [ ] **RED-4**: Test `list_by_token_budget` con presupuesto cero devuelve vacío
- [ ] **RED-5**: Test `list_by_token_budget` con conversación vacía devuelve vacío
- [ ] **RED-6**: Test orquestador usa `list_by_token_budget` en lugar de `SessionWindow`
- [ ] **RED-7**: Verificar que `cargo test` falla para los nuevos tests

## GREEN (Implement minimal code)

- [ ] **GREEN-1**: Implementar `MessagesRepo::list_by_token_budget()` con SQL window function
- [ ] **GREEN-2**: Modificar `Orchestrator::process_message()` para usar `list_by_token_budget()`
- [ ] **GREEN-3**: Modificar `Orchestrator::process_message_stream()` para usar `list_by_token_budget()`
- [ ] **GREEN-4**: Eliminar `SessionWindow` struct y `session_window.rs`
- [ ] **GREEN-5**: Eliminar `session_window` del estado del `Orchestrator`
- [ ] **GREEN-6**: Eliminar `session_window` de `pub mod` en `orchestrator/mod.rs`
- [ ] **GREEN-7**: Verificar `cargo test` pasa (100% green)

## REFACTOR (Clean & consolidate)

- [ ] **REFAC-1**: Ejecutar `cargo clippy -- -D warnings` — cero warnings
- [ ] **REFAC-2**: Ejecutar `cargo fmt --check` — formato correcto
- [ ] **REFAC-3**: Verificar que no queda código muerto (imports, referencias a SessionWindow)
- [ ] **REFAC-4**: Verificar que `cargo test` sigue en verde tras refactor

## ARCHIVE

- [ ] **ARCHIVE-1**: Ejecutar `openspec archive sql-window-history`
- [ ] **ARCHIVE-2**: Verificar que los specs se consolidaron en `openspec/specs/`