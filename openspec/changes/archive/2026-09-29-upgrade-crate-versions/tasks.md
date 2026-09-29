# TDD Task Checklist: upgrade-crate-versions

## 1. RED - Confirmar el fallo actual
- [x] 1.1 Ejecutar `cargo check --all-targets` y registrar los 7 errores de `sqlx 0.9`
- [x] 1.2 Ejecutar `cargo test` y registrar los 12 tests que fallan por `axum 0.8`

## 2. GREEN - Adaptar el código a los breaking changes
- [x] 2.1 `src/db/repos/reminders.rs` — envolver SQL dinámico con `sqlx::AssertSqlSafe`
- [x] 2.2 `src/db/repos/shopping_list.rs` — envolver SQL dinámico con `sqlx::AssertSqlSafe` (2 sitios)
- [x] 2.3 `src/db/repos/stats.rs` — envolver `query_scalar` dinámico con `sqlx::AssertSqlSafe`
- [x] 2.4 `src/db/repos/tasks.rs` — envolver SQL dinámico con `sqlx::AssertSqlSafe`
- [x] 2.5 `src/routes/export.rs` — envolver SQL dinámico con `sqlx::AssertSqlSafe`
- [x] 2.6 `src/tools/unified_search.rs` — envolver SQL dinámico con `sqlx::AssertSqlSafe`
- [x] 2.7 `src/lib.rs` — migrar 3 rutas `:param` → `{param}`
- [x] 2.8 `src/routes/tasks.rs` — migrar ruta `:id` → `{id}`
- [x] 2.9 `src/routes/events.rs` — migrar ruta `:id` → `{id}`
- [x] 2.10 `src/routes/stream.rs` — migrar ruta `:request_id` → `{request_id}`
- [x] 2.11 Ejecutar `cargo check --all-targets` (sin errores)
- [x] 2.12 Ejecutar `cargo test` (100% verde)

## 3. REFACTOR - Limpiar y verificar
- [x] 3.1 Ejecutar `cargo clippy -- -D warnings` (cero warnings)
- [x] 3.2 Ejecutar `cargo fmt --check`
- [x] 3.3 Verificar que no quedan rutas con sintaxis `:param` (`rg '"/[^"]*:[a-z_]+' src/`)
- [x] 3.4 Verificar que no quedan `sqlx::query(&` con `&String` dinámico
- [x] 3.5 Ejecutar tests de integración (`cargo test --tests`)
- [x] 3.6 Archivar change proposal con `openspec archive upgrade-crate-versions`