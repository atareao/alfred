# TDD Tasks — Real SSE Streaming

## Fase 1: Tests para OpenRouter streaming (RED)

- [x] **1.1** Test: `chat_stream` con texto plano emite `StreamEvent::Chunk` con cada delta
- [x] **1.2** Test: `chat_stream` emite `StreamEvent::Done` al final con `ChatResponse` y `usage`
- [x] **1.3** Test: `chat_stream` con tool calls en delta emite `StreamEvent::ToolCall`
- [x] **1.4** Test: `chat_stream` con tool calls emite `StreamEvent::Done` incluyendo tool calls
- [x] **1.5** Test: `chat_stream` con error HTTP retorna `Err(LLMError::HttpError)`
- [x] **1.6** Test: `chat_stream` ignora chunks vacíos (`data: {}`)
- [x] **1.7** Test: `chat_stream` con `stream=false` equivale a `chat()` (un solo `Done`)

## Fase 2: Implementar streaming en OpenRouter (GREEN)

- [x] **2.1** Implementar `parse_sse_event()` — parsea una línea `data: {...}` a delta
- [x] **2.2** Implementar `chat_stream()` real con `reqwest` + `response.chunk()` + SSE
- [x] **2.3** Enviar `"stream": true` en el body de la petición
- [x] **2.4** Emitir `StreamEvent::Chunk` por cada delta de contenido
- [x] **2.5** Emitir `StreamEvent::ToolCall` cuando aparecen tool_calls en el stream
- [x] **2.6** Emitir `StreamEvent::Done` al recibir `finish_reason`
- [x] **2.7** Preservar `usage` del último chunk para el Done

## Fase 3: Adaptar orquestador (RED primero)

- [x] **3.1** Mocks LLM actualizados con `chat_stream()` real
- [x] **3.2** `process_message_stream` usa `chat_stream()` en todas las iteraciones
- [x] **3.3** Chunks reenviados como `SSEEvent::Chunk` al frontend

## Fase 4: Implementar uso de streaming en orquestador (GREEN)

- [x] **4.1** En `process_message_stream`, última iteración usa `chat_stream()` en vez de `chat()`
- [x] **4.2** Reenviar `StreamEvent::Chunk` como `SSEEvent::Chunk` al frontend
- [x] **4.3** Acumular chunks en buffer para persistir mensaje completo
- [x] **4.4** Manejar `StreamEvent::ToolCall` como tool call normal
- [x] **4.5** Manejar `StreamEvent::Done` con/sin tool calls

## Fase 5: Refactor y limpieza

- [x] **5.1** Eliminar el hack `chat_stream()` actual que llama a `chat()` internamente
- [x] **5.2** Verificar que no hay warnings de clippy
- [x] **5.3** Ejecutar `cargo test` completo (verde)
- [x] **5.4** Ejecutar `cargo clippy -- -D warnings` (limpio)