# Tasks

## 1. CollapseWorker — umbral desde config

- [x] 1.1 RED: test que verifique que `create_message` usa `config.collapse_threshold_tokens` (p. ej. umbral 500 → mensaje de ~600 tokens dispara el canal).
- [x] 1.2 GREEN: pasar `config.collapse_threshold_tokens` a `MessagesRepo::create` en `handlers/messages.rs` y `orchestrator/agent.rs`.

## 2. CollapseWorker — no descartar IDs en silencio

- [x] 2.1 RED: test que verifique que un canal lleno no pierde el ID (o que se registra un warning).
- [x] 2.2 GREEN: sustituir `try_send` por backpressure (`tokio::spawn` + `send().await`) con fallback a `try_send` + warning.

## 3. EpisodicMemoryWorker — cooldown según spec

- [x] 3.1 RED: test que verifique cooldown = `max(poll_interval/2, 30s)`.
- [x] 3.2 GREEN: calcular el cooldown desde `config.poll_interval_minutes`.

## 4. EpisodicMemoryWorker — sin bucle de coste tras fallo de persist

- [x] 4.1 RED: test que verifique que un fallo de `persist()` no provoca una segunda llamada al LLM dentro del cooldown.
- [x] 4.2 GREEN: marcar el rate-limiter también cuando `persist()` falla.

## 5. EpisodicMemoryWorker — persistencia atómica

- [x] 5.1 RED: test que verifique que un fallo al insertar en `vec_memory` no deja fila huérfana en `memory`.
- [x] 5.2 GREEN: envolver los inserts de `memory` + `vec_memory` en una transacción.

## 6. EpisodicMemoryWorker — rate-limiter por instancia

- [x] 6.1 REFACTOR: sustituir `static LAST_LLM_ATTEMPT` por estado por-instancia (`Arc<AtomicI64>` o campo de struct); retirar `#[serial]` y resets manuales de los tests.

## 7. Stats con profile_id NULL

- [x] 7.1 RED: test que verifique que `record_request` de los workers inserta la fila con `profile_id = NULL` (la FK no falla y la fila existe).
- [x] 7.2 GREEN: cambiar `StatsRepo::record_request` a `profile_id: Option<&str>` y pasar `None` desde `collapse.rs` y `episodic_memory.rs` (sustituir `"background"`/`"episodic"`).

## 8. Verificación

- [x] 8.1 `cargo fmt --check` limpio.
- [x] 8.2 `cargo clippy --all-targets -- -D warnings` limpio.
- [x] 8.3 `cargo test` verde.
- [x] 8.4 `openspec validate fix-remaining-workers --strict` válido.
