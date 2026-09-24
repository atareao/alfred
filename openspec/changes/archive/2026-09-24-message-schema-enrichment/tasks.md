# Tasks: Message Schema Enrichment

## Phase 2 — TDD Checklist

### RED — Escribir tests fallantes

- [x] **RED 1**: `estimate_tokens` — test unitario para `test_estimate_tokens_empty`, `test_estimate_tokens_short`, `test_estimate_tokens_long`
- [x] **RED 2**: `Message` model — test de construcción con nuevos campos (tokens_count, collapsed_content, etc.)
- [x] **RED 3**: `MessagesRepo::create` — test que verifica que se calcula `tokens_count` automáticamente
- [x] **RED 4**: `MessagesRepo::create` — test que verifica que mensajes cortos NO encolan collapse
- [x] **RED 5**: `MessagesRepo::create` — test que verifica que mensajes largos SÍ encolan collapse
- [x] **RED 6**: `Config` — test del nuevo campo `collapse_threshold_tokens` con default y custom
- [x] **RED 7**: Schema migration — test que verifica que `run_migrations` añade las columnas correctamente

### GREEN — Implementación mínima

- [x] **GREEN 1**: Implementar `estimate_tokens` en `src/models/message.rs`
- [x] **GREEN 2**: Añadir campos a `Message` struct
- [x] **GREEN 3**: Migration SQL en `src/db/schema.rs` (ALTER TABLE)
- [x] **GREEN 4**: Actualizar `MessagesRepo::create` para computar tokens y encolar collapse si corresponde
- [x] **GREEN 5**: Actualizar `MessagesRepo::find_by_id` para SELECT de nuevas columnas
- [x] **GREEN 6**: Actualizar `MessagesRepo::list_by_conversation` para SELECT de nuevas columnas
- [x] **GREEN 7**: Actualizar `MessagesRepo::list_recent` para SELECT de nuevas columnas
- [x] **GREEN 8**: Actualizar `Config` con `collapse_threshold_tokens`
- [x] **GREEN 9**: Crear worker stub para collapse (módulo workers/collapse.rs)
- [x] **GREEN 10**: Integrar threshold config en handler de creación de mensajes

### REFACTOR — Limpieza

- [x] **REFACTOR 1**: `cargo clippy -- -D warnings` pasa limpio
- [x] **REFACTOR 2**: `cargo test` todo en verde
- [x] **REFACTOR 3**: `cargo fmt --check` pasa
- [x] **REFACTOR 4**: Consolidar y archivar change proposal