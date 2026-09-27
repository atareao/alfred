# Memory Wiring Fix — Conectar memory_tx en producción

## Intent

Corregir el wiring que impide que la memoria episódica funcione en producción.
Actualmente `AppState::new_with_orchestrator()` devuelve `memory_tx: None`, el
`Orchestrator` no tiene `memory_tx`, y `main.rs` nunca inicia el `WorkerPool`.
Como resultado, aunque el handler `create_message` envía señal por `memory_tx`,
nunca llega al `EpisodicMemoryWorker`.

## Scope

### Incluye

1. Añadir `memory_tx: Option<mpsc::Sender<()>>` al `Orchestrator`
2. Enviar señal por `memory_tx` desde el agente tras persistir mensajes (user + assistant)
3. Conectar `WorkerPool.memory_tx` a `AppState.memory_tx` en el flujo de producción
4. Modificar `AppState::new_with_orchestrator()` para iniciar `WorkerPool` y pasar `memory_tx`
5. Modificar `main.rs` para pasar `WorkerPool.memory_tx` a `AppState`

### No incluye

- Cambios en el handler `create_message` (ya funciona correctamente)
- Cambios en `EpisodicMemoryWorker` (ya funciona correctamente)
- Cambios en `WorkerPool::start()` (ya crea el canal correctamente)

## Impacto

### Archivos modificados

- `src/orchestrator/agent.rs` — añadir `memory_tx` al `Orchestrator`, enviar señal
- `src/lib.rs` — `AppState::new_with_orchestrator()` inicia `WorkerPool` y pasa `memory_tx`
- `src/main.rs` — usar `WorkerPool` y pasar `memory_tx` a `AppState`

### Tests afectados

- `src/orchestrator/agent.rs` — test de agente envía signal por `memory_tx` (RED)
- `workers/pool.rs` — tests existentes siguen en GREEN
- `lib.rs` — test de producción verifica `memory_tx` es `Some`