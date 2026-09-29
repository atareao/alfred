# Cleanup: clippy lints y dependencia muerta (2026-09)

## Intent
Dejar la suite de tests y el código de test libres de lints de `clippy --all-targets`
y eliminar una dependencia de desarrollo que no se usa, reduciendo el árbol de
dependencias transitivas.

## Problema actual
- `cargo clippy --all-targets -- -D warnings` reportaba **14 lints** en código de test
  y en módulos de producción (imports sin usar, `assert_eq!(x, false)`, `loop { match }`,
  `MutexGuard` retenido a través de `.await`, variable duplicada, parámetro sin usar).
- `axum-test-helper = "0.4"` estaba declarado en `[dev-dependencies]` pero **no se usa**
  en ningún test. Arrastraba versiones antiguas de `axum 0.7.9`, `reqwest 0.11.27`,
  `tower 0.4.13` y `base64 0.21.7` al `Cargo.lock`.

## Scope
### Lints corregidos
- Imports sin usar: `axum::http::StatusCode`, `serde_json::json`, `tower::ServiceExt`
  (`src/handlers/stats.rs`), `use super::*` (`src/tools/geo_utils.rs`),
  `sqlx::SqlitePool` (`src/workers/collapse.rs`, `src/workers/pool.rs`).
- `dflt` → `_dflt` (`src/db/schema.rs`).
- `let db` duplicado eliminado (`src/workers/episodic_memory.rs`).
- `MutexGuard` acotado para soltarse antes del `.await` (`src/workers/episodic_memory.rs`).
- `assert_eq!(x, false)` → `assert!(!x)` (`src/db/repos/events.rs`).
- `loop { match }` → `while let` (`src/orchestrator/agent.rs`).
- Código muerto en tests (`tests/api/common/mod.rs`, `tests/db/stats_migration.rs`).

### Dependencias
- Eliminar `axum-test-helper` de `[dev-dependencies]` en `Cargo.toml`.
- Regenerar `Cargo.lock` (queda con una sola versión de axum/reqwest/tower/base64).

## Impacto
- **Sin cambios de comportamiento**: solo limpieza de lints y dependencias.
- `Cargo.lock` está en `.gitignore`; se regenera localmente.
- Verificado: `clippy --all-targets` 0 errores, `cargo test` 599 tests en verde,
  `cargo fmt --check` OK.

## Nota de protocolo
Este change se registra **retroactivamente**: los cambios de código se aplicaron antes
de crear el proposal. Se archiva con `--skip-specs` por ser limpieza de tooling/tests
sin cambio de contrato.
