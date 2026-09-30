# Proposal: Arreglar los workers que se quedan (Collapse + EpisodicMemory)

## Why

Tras eliminar los cuatro workers proactivos no funcionales (`remove-proactive-workers`), quedan dos workers reales con bugs concretos detectados en revisión:

**CollapseWorker**
- El umbral `collapse_threshold_tokens` es decorativo: los dos callers pasan `2000` hardcodeado a `MessagesRepo::create`, así que `COLLAPSE_THRESHOLD_TOKENS` no tiene efecto.
- `try_send` descarta el ID en silencio si el canal (256) está lleno → el mensaje nunca se colapsa, sin log ni backpressure.

**EpisodicMemoryWorker**
- Fuga de coste: si el LLM acierta pero `persist()` falla, no se marca el rate-limiter → el siguiente poll vuelve a llamar al LLM en bucle.
- `memory` y `vec_memory` se insertan en dos pasos no transaccionales → fila huérfana si falla el segundo.
- El cooldown del rate-limiter es `60s` fijo, pero la spec exige `max(poll_interval/2, 30s)`.
- El rate-limiter es un `static AtomicI64` global compartido entre instancias (frágil, obliga a `#[serial]` en tests).

**Stats con `profile_id` falso**
- `llm_requests.profile_id` es `TEXT REFERENCES profiles(id)` y las FK están activas (`foreign_keys(true)`). Los workers insertan `"background"`/`"episodic"`, que no existen en `profiles` → la FK falla y el `let _ =` se traga el error: **las stats de los workers nunca se guardan**.
- Las llamadas de `collapse`/`episodic` son operaciones de sistema, no del usuario. La columna ya es nullable, así que lo correcto es registrar `NULL`.

## What Changes

- **Collapse**: leer el umbral de `config.collapse_threshold_tokens` en `handlers/messages.rs` y `orchestrator/agent.rs`.
- **Collapse**: no descartar IDs en silencio cuando el canal está lleno (backpressure vía `send().await` en una tarea, o log de warning si no hay runtime).
- **Episodic**: aplicar el cooldown también tras un fallo de `persist()` para evitar el bucle de coste.
- **Episodic**: persistir `memory` + `vec_memory` de forma atómica (transacción).
- **Episodic**: calcular el cooldown como `max(poll_interval_minutes * 60 / 2, 30)` para cumplir la spec.
- **Episodic**: sustituir el `static LAST_LLM_ATTEMPT` por estado por-instancia.
- **Stats**: registrar `llm_requests` con `profile_id = NULL` (operación de sistema) en vez de literales que violan la FK.

## Capabilities

### New Capabilities

<!-- Ninguna -->

### Modified Capabilities

- `workers`: umbral de colapso desde config; no-descarte; no-reintento-LLM tras fallo de persistencia; persistencia atómica; stats con `profile_id = NULL`.

## Impact

- **Editados**: `src/handlers/messages.rs`, `src/orchestrator/agent.rs`, `src/workers/collapse.rs`, `src/workers/episodic_memory.rs`, `src/workers/pool.rs`, `src/db/repos/stats.rs`.
- **Specs**: `workers`.
- **Sin cambios** en API pública, schema ni dependencias.

## Fuera de alcance (change posterior)

- **`wire-rag`**: conectar el pipeline RAG. Hoy `ContextBuilder` se crea con `pool: None, provider: None` (`lib.rs:189`) y `build_rag_memories` devuelve siempre `["memory1", "memory2"]` hardcodeado; `search_by_vector` nunca se ejecuta en producción; el módulo `src/embeddings/` está muerto. Se abordará en su propio change, donde tendrán sentido las decisiones sobre modelo/dimensión de embedding, `sqlite-vec` y `profile_id` en `memory`.
