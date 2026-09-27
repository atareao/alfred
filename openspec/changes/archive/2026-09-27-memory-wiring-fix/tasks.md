# Tasks: Memory Wiring Fix

## TDD Task Checklist

### Fase 1: Orchestrator memory_tx
- [x] 1.1 RED: test de Orchestrator envía signal por memory_tx al persistir mensaje de usuario
- [x] 1.2 RED: test de Orchestrator envía signal por memory_tx al persistir mensaje de asistente
- [x] 1.3 GREEN: añadir `memory_tx` al struct Orchestrator + constructor
- [x] 1.4 GREEN: enviar signal tras persistir mensaje de usuario (línea ~828)
- [x] 1.5 GREEN: enviar signal tras persistir mensaje de asistente (línea ~1107)
- [x] 1.6 REFACTOR: verificar `cargo clippy -- -D warnings` y `cargo test`

### Fase 2: AppState production wiring
- [x] 2.1 RED: test de `AppState::new_with_orchestrator()` verifica `memory_tx` es `Some`
- [x] 2.2 GREEN: modificar `AppState::new_with_orchestrator()` para crear WorkerPool y exponer memory_tx
- [x] 2.3 GREEN: actualizar `main.rs` para que pase Config a new_with_orchestrator
- [x] 2.4 REFACTOR: verificar `cargo clippy -- -D warnings` y `cargo test` completo

### Fase 3: Verificación final
- [x] 3.1 Ejecutar suite completa: `cargo test --package alfred`
- [x] 3.2 Ejecutar linter: `cargo clippy --all-targets -- -D warnings`
- [ ] 3.3 Archivar change proposal con `openspec archive memory-wiring-fix`