# Real SSE Streaming para OpenRouter

## Why

El streaming SSE actual es falso: `chat_stream()` en `OpenRouterProvider` llama internamente a `chat()` (no-streaming, `"stream": false`). El cliente HTTP espera a recibir el body JSON completo antes de devolver nada, generando ~11s de latencia para respuestas de ~330 tokens. El usuario ve silencio absoluto durante ese tiempo.

Además, el orquestador (`process_message_stream`) usa `self.llm.chat()` en todas las iteraciones del ReAct loop, ignorando completamente la capacidad de streaming del provider.

## What Changes

1. **`src/llm/openrouter.rs`** — Se implementa `chat_stream()` real:
   - Envía `"stream": true` en el body
   - Lee la respuesta HTTP como stream de bytes con `response.chunk()`
   - Procesa líneas SSE línea por línea
   - Acumula tool_calls parciales entre chunks (OpenRouter los envía fragmentados)
   - Emite `StreamEvent::Chunk`, `StreamEvent::ToolCall`, y `StreamEvent::Done`

2. **`src/llm/openrouter.rs`** — Nueva función `parse_sse_event()` que convierte una línea SSE en un `StreamEvent` opcional, usando un `StreamAccumulator` para estado entre chunks.

3. **`Cargo.toml`** — Se añade feature `"stream"` a reqwest (necesaria para `response.chunk()`).

4. **`src/orchestrator/agent.rs`** — `process_message_stream()` ahora usa `chat_stream()` en todas las iteraciones del ReAct loop:
   - `StreamEvent::Chunk` se reenvía como `SSEEvent::Chunk` al frontend
   - `StreamEvent::ToolCall` se acumula para procesar al recibir Done
   - `StreamEvent::Done` con tool_calls → ejecuta tools y continúa el loop
   - `StreamEvent::Done` sin tool_calls → persiste y emite evento final

5. **Tests** — 7 tests unitarios nuevos para parseo SSE + integración en el orquestador. Todos los mocks LLM actualizados para implementar `chat_stream()`.

## Impacto

- **Percepción UX**: De ~11s de silencio a ~500ms hasta el primer token
- **Arquitectura**: El ReAct loop sigue siendo síncrono; solo la entrega de la respuesta final es streaming
- **Sin cambios**: API, frontend, base de datos — todo permanece igual