# llm/provider Specification — Streaming delta

## ADDED Requirements

### Requirement: OpenRouter chat_stream produces real SSE events

**Given** un `OpenRouterProvider` configurado  
**When** se llama a `chat_stream()` con `ChatRequest { stream: true }`  
**Then** envía `"stream": true` en el body de la petición a OpenRouter  
**And** parsea el stream SSE línea por línea  
**And** emite `StreamEvent::Chunk` por cada delta de `content`  
**And** emite `StreamEvent::Done` con el `ChatResponse` final incluyendo `usage`  
**And** si aparecen `tool_calls` en el stream, emite `StreamEvent::ToolCall`

#### Scenario: Stream de texto plano (sin tool calls)
**Given** una respuesta SSE de OpenRouter con solo `choices[0].delta.content`  
**When** `chat_stream()` procesa el stream  
**Then** emite `StreamEvent::Chunk(texto)` por cada fragmento  
**And** emite `StreamEvent::Done(ChatResponse { message.content: texto_completo, tool_calls: None })` al final  
**And** el `ChatResponse` final incluye `usage` con tokens del último evento

#### Scenario: Stream con tool call en delta
**Given** una respuesta SSE de OpenRouter  
**When** `choices[0].delta.tool_calls` aparece en el stream  
**Then** acumula los tool calls parcialmente (pueden venir en múltiples chunks)  
**And** al recibir `finish_reason: "tool_calls"`, emite `StreamEvent::ToolCall`  
**And** emite `StreamEvent::Done` con el `ChatResponse` incluyendo los tool calls

#### Scenario: Error en el stream HTTP
**Given** una petición SSE a OpenRouter  
**When** la conexión HTTP falla (timeout, reset, etc.)  
**Then** retorna `Err(LLMError::HttpError)`  
**And** cierra el stream

#### Scenario: chunk vacío se ignora
**Given** un stream SSE de OpenRouter  
**When** se recibe un `data: {}` sin content ni tool_calls  
**Then** no emite ningún evento  
**And** continúa con el siguiente chunk

#### Scenario: chat_stream con stream=false
**Given** un `ChatRequest` con `stream: false`  
**When** se llama a `chat_stream()`  
**Then** el comportamiento puede ser equivalente a `chat()`  
**And** emite un único `StreamEvent::Done` con la respuesta completa