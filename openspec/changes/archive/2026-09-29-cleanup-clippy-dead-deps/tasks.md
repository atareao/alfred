# TDD Task Checklist: cleanup-clippy-dead-deps

> **Nota:** change registrado retroactivamente. El código ya estaba aplicado y verificado
> antes de crear este proposal; se documenta y archiva para cerrar el hueco de protocolo.

## 1. RED - Reproducir los lints
- [x] 1.1 Ejecutar `cargo clippy --all-targets -- -D warnings` y registrar los 14 lints
- [x] 1.2 Confirmar que `axum-test-helper` no se usa en ningún test

## 2. GREEN - Corregir
- [x] 2.1 Eliminar imports sin usar en `src/handlers/stats.rs`, `src/tools/geo_utils.rs`,
      `src/workers/collapse.rs`, `src/workers/pool.rs`
- [x] 2.2 Corregir `dflt` → `_dflt` en `src/db/schema.rs`
- [x] 2.3 Eliminar `let db` duplicado y acotar `MutexGuard` en `src/workers/episodic_memory.rs`
- [x] 2.4 `assert_eq!(x, false)` → `assert!(!x)` en `src/db/repos/events.rs`
- [x] 2.5 `loop { match }` → `while let` en `src/orchestrator/agent.rs`
- [x] 2.6 Limpiar código muerto en `tests/api/common/mod.rs` y `tests/db/stats_migration.rs`
- [x] 2.7 Eliminar `axum-test-helper` de `Cargo.toml` y regenerar `Cargo.lock`

## 3. REFACTOR - Verificar
- [x] 3.1 `cargo clippy --all-targets -- -D warnings` → 0 errores
- [x] 3.2 `cargo test` → 599 tests en verde
- [x] 3.3 `cargo clippy -- -D warnings` → 0 warnings
- [x] 3.4 `cargo fmt --check` → OK
- [x] 3.5 Archivar change proposal con `openspec archive --skip-specs`
