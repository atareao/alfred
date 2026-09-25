# orchestrator/agent Specification — Streaming delta

## ADDED Requirements

### Requirement: Orchestrator uses chat_stream for incremental response delivery

**Given** el orquestador procesando `process_message_stream()`  
**When** se alcanza la iteración final del ReAct loop (sin tool calls)  
**Then** usa `self.llm.chat_stream()` para recibir la respuesta incrementalmente  
**And** emite cada chunk como `SSEEvent::Chunk` al frontend vía `tx.send()`  
**And** al finalizar, emite `SSEEvent::Done` con los IDs

#### Scenario: Chunks se reenvían como SSEEvent::Chunk
**Given** el orquestador ha recibido `StreamEvent::Chunk("Hola")` del LLM  
**When** procesa el evento  
**Then** envía `SSEEvent::Chunk { content: "Hola" }` por el canal SSE  
**And** acumula el contenido para el mensaje final a persistir

#### Scenario: Tool call en stream (caso borde)
**Given** el orquestador en la iteración final  
**When** el stream devuelve `StreamEvent::ToolCall`  
**Then** el orquestador cambia a modo tool call  
**And** procesa el tool call normalmente  
**And** continúa el ReAct loop (no emite Done)

#### Scenario: Done con tool_calls presente
**Given** el orquestador recibe `StreamEvent::Done(response)`  
**When** `response.message.tool_calls` es `Some`  
**Then** NO emite los chunks acumulados como SSEEvent::Chunk  
**And** procesa los tool calls en el ReAct loop  
**And** continúa a la siguiente iteración

#### Scenario: Done con respuesta final sin tool calls
**Given** el orquestador recibe `StreamEvent::Done(response)`  
**When** `response.message.tool_calls` es `None`  
**Then** emite los chunks acumulados como `SSEEvent::Chunk`  
**And** persiste el mensaje en DB  
**And** emite `SSEEvent::Done` con los IDs

#### Scenario: Fallback si chat_stream() no está implementado
**Given** un provider sin implementación real de `chat_stream()`  
**When** el orquestador llama a `chat_stream()`  
**Then** recibe un stream con un único `StreamEvent::Done`  
**And** el comportamiento es equivalente al actual