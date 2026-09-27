# Stats Recording (LLM request tracking)

## Intent

Las estadísticas del dashboard (`/stats`) siempre muestran ceros porque **nadie escribe en la tabla `llm_requests`**. El `StatsRepo` solo tiene métodos de lectura (`summary`, `by_model`, `by_day`, `tools_summary`, etc.) y limpieza (`purge_old`), pero no hay ningún método público para insertar registros cuando se hace una llamada LLM.

La función `insert_request` solo existe en bloques `#[cfg(test)]` — no es accesible en producción.

Este cambio añade:
1. Un método público `StatsRepo::record_request()` para insertar registros en `llm_requests`
2. La llamada a `record_request` desde el `Orchestrator` tras cada `self.llm.chat()` en ambos flujos (`process_message` y `process_message_stream`)
3. Los campos `model` y `duration_ms` que actualmente no se registran

## Scope

### Incluye
1. Método `StatsRepo::record_request()` — público, inserta un registro completo en `llm_requests`
2. Hook en `Orchestrator::process_message()` — registrar tras cada llamada LLM en el ReAct loop
3. Hook en `Orchestrator::process_message_stream()` — registrar en el `StreamEvent::Done` (el `ChatResponse` del final tiene `usage`)
4. El campo `model` se obtiene del `OpenRouterConfig.model`
5. El campo `duration_ms` se calcula midiendo el tiempo de la llamada al provider
6. Los tool_calls se serializan desde el `ChatRequest`/`ChatResponse` para poblar la columna `tool_calls`
7. El `profile_id` se pasa desde el Orchestrator (ya está disponible en ambos métodos)

### No incluye
- Parseo de `cached_tokens`, `reasoning_tokens` o `cost` desde OpenRouter (de momento se guardan como 0; OpenRouter los devuelve pero requeriría extender `TokenUsage` y `ChatResponse`)
- `cache_hit` (se deja como 0)
- `is_byok` (se deja como 0)
- Provider name (se deja como NULL, se puede poblar después)

## Impacto

### Código nuevo
- `StatsRepo::record_request()` en `src/db/repos/stats.rs`

### Código modificado
- `src/orchestrator/agent.rs` — ambos flujos registrarán stats tras cada llamada LLM

### Tests nuevos
- `StatsRepo::record_request` unit tests en `src/db/repos/stats.rs`
- Orchestrator tests adaptados (o nuevos) para verificar que se registran las stats