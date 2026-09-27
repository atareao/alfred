# Tasks: Stats Recording (LLM request tracking)

## TDD Task Checklist

### Fase 1: StatsRepo::record_request
- [x] 1.1 RED: test para `StatsRepo::record_request` inserta registro exitoso
- [x] 1.2 GREEN: implementar `StatsRepo::record_request` público
- [x] 1.3 RED: test para `record_request` con tool_calls
- [x] 1.4 RED: test para `record_request` con error
- [x] 1.5 RED: test para `record_request` asigna created_at automáticamente
- [x] 1.6 GREEN: verificar que los 3 tests adicionales pasan con la misma implementación

### Fase 2: Hook en Orchestrator::process_message (no-streaming)
- [x] 2.1 RED: test de `process_message` verifica que se inserta 1 fila en `llm_requests` por llamada LLM
- [x] 2.2 GREEN: hookear `StatsRepo::record_request` en `process_message` tras cada `self.llm.chat()`

### Fase 3: Hook en Orchestrator::process_message_stream (streaming)
- [x] 3.1 RED: test de `process_message_stream` verifica que se inserta 1 fila en `llm_requests`
- [x] 3.2 GREEN: hookear `StatsRepo::record_request` en `process_message_stream` en `StreamEvent::Done`

### Fase 4: Limpieza y verificación final
- [x] 4.1 Ejecutar `cargo clippy -- -D warnings`
- [x] 4.2 Ejecutar `cargo test` — todos los tests pasan
- [x] 4.3 Archivar change proposal con `openspec archive stats-recording`