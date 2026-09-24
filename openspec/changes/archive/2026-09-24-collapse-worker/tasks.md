# Tasks: Collapse Worker

## RED — Escribir tests fallantes

- [x] **RED 1**: `CollapseWorker` — test que verifica que procesa un mensaje del canal y llama al LLM
- [x] **RED 2**: `CollapseWorker` — test que verifica que actualiza `collapsed_content` en DB
- [x] **RED 3**: `Config` — test para `collapse_model` default y custom
- [x] **RED 4**: `SettingsRepo` — test para `collapse_prompt` seed default
- [x] **RED 5**: `WorkerPool` — test que verifica que incluye collapse worker
- [x] **RED 6**: Handler — test que verifica que mensajes largos encolan collapse

## GREEN — Implementación mínima

- [x] **GREEN 1**: Crear `src/workers/collapse.rs` con `CollapseWorker`
- [x] **GREEN 2**: Implementar lógica de lectura de mensaje + llamada LLM + update DB
- [x] **GREEN 3**: Añadir `collapse_model` a `Config`
- [x] **GREEN 4**: Seed `collapse_prompt` default en `SettingsRepo::seed_defaults`
- [x] **GREEN 5**: Añadir collapse worker a `WorkerPool`
- [x] **GREEN 6**: Añadir collapse channel a `AppState`
- [x] **GREEN 7**: Conectar callback en handler de creación de mensajes

## REFACTOR — Limpieza

- [x] **REFACTOR 1**: `cargo clippy -- -D warnings` pasa limpio
- [x] **REFACTOR 2**: `cargo test` todo en verde
- [x] **REFACTOR 3**: `cargo fmt --check` pasa
- [x] **REFACTOR 4**: Archivar change proposal