# Upgrade Crate Versions (2026-09)

## Intent
Actualizar las dependencias Rust a sus últimas versiones mayores y adaptar el código a los
breaking changes, dejando el proyecto compilando y con la suite de tests en verde.

## Problema actual
`Cargo.toml` fue actualizado a versiones mayores. El proyecto **no compila** y **12 tests fallan**:

- **7 errores de compilación** por `sqlx 0.9`: el nuevo trait `SqlSafeStr` rechaza SQL dinámico
  (`&String`) en `sqlx::query()` / `sqlx::query_scalar()`.
- **12 tests fallan** por `axum 0.8`: la sintaxis de rutas `:param` ya no es válida y provoca
  panic al construir el router (`Path segments must not start with ':'`).

## Scope
### Dependencias actualizadas
| Crate | Antes | Ahora |
|---|---|---|
| axum | 0.7 | 0.8 |
| sqlx | 0.8 | 0.9 |
| tower-http | 0.5 | 0.7 |
| thiserror | 1 | 2 |
| reqwest | 0.12 | 0.13 |
| base64 | 0.22 | 0.23 |
| serial_test | 3 | 4 |
| tower | 0.4 | 0.5 |

### Adaptaciones de código
- **sqlx 0.9** — envolver SQL dinámico con `sqlx::AssertSqlSafe(...)`:
  - `src/db/repos/reminders.rs:71`
  - `src/db/repos/shopping_list.rs:36,42`
  - `src/db/repos/stats.rs:176`
  - `src/db/repos/tasks.rs:101`
  - `src/routes/export.rs:36`
  - `src/tools/unified_search.rs:33`
- **axum 0.8** — migrar rutas `:param` → `{param}`:
  - `src/lib.rs:331` `/api/messages/:msg_id`
  - `src/lib.rs:344` `/api/memories/:id`
  - `src/lib.rs:347` `/api/tools/:id/toggle`
  - `src/routes/tasks.rs:14` `/api/tasks/:id`
  - `src/routes/events.rs:14` `/api/events/:id`
  - `src/routes/stream.rs:186` `/api/approval/:request_id`
- **reqwest 0.13** — feature `rustls-tls` renombrada a `rustls` (ya aplicado en `Cargo.toml`).

## Impacto
- **API REST sin cambios de contrato**: los paths públicos son idénticos; solo cambia la sintaxis
  interna del router de axum.
- **Sin cambios en frontend** (`frontend/package.json` intacto).
- `Cargo.lock` está en `.gitignore`; se regenera localmente.
- Riesgo: comportamiento de `sqlx`/`axum` en runtime. Mitigado por la suite de 510 tests.
