# Change Proposal: Events REST API

## Why

El `CalendarView` del frontend llama a endpoints REST (`/api/events`) para
listar, crear, editar y eliminar eventos del calendario. Estos endpoints no
existen en el backend — el `EventsRepo` está implementado pero no hay handlers
HTTP ni rutas registradas. Como resultado, el frontend recibe 404 y la agenda
no muestra nada aunque haya eventos en la BD.

## What Changes

- Nuevo **handler** `src/handlers/events.rs` con 4 funciones:
  - `list_events` — GET con query params `start`/`end`
  - `create_event` — POST con body JSON
  - `update_event` — PUT con `:id` y body JSON
  - `delete_event` — DELETE con `:id`
- Nueva **ruta** `src/routes/events.rs` que enlaza handlers a rutas HTTP
- Registro en `src/routes/mod.rs` y `src/lib.rs` (`app_with_state()`)
- **Tests** de integración en `tests/api/events.rs` (9 escenarios)
- Nuevo variant `UnprocessableEntity` en `AppError` para devolver 422

## Scope
- Nuevo handler: `src/handlers/events.rs`
- Nueva ruta: `src/routes/events.rs`
- Registrar en `src/routes/mod.rs` y `src/lib.rs`
- Tests de integración en `tests/api/events.rs`

## Impact
- No breaking changes. Las rutas son nuevas.
- El `CalendarView` del frontend empezará a funcionar sin cambios en frontend.

## Approach
Seguir el patrón exacto de `routes/tools.rs` + `handlers/tools.rs`:
- Handler recibe `State<AppState>`, devuelve `Result<Json<T>, AppError>`
- Ruta define `get`, `post`, `put`, `delete`
- Se registra en `app_with_state()` en `lib.rs`