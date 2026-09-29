# orchestrator/agent Specification

## Purpose
Bucle ReAct del agente de Valet: uso del system prompt desde la base de datos, tolerancia a errores de herramientas, límite de reintentos, entrega incremental vía chat_stream, prioridad de los tool_calls del evento Done y registro de estadísticas de cada llamada al LLM.

## Requirements

### Requirement: System prompt template uses Markdown and emojis
**Given** una base de datos migrada con el prompt sembrado en `settings.system_prompt`
**When** el orquestador construye la petición al LLM
**Then** SHALL leer `system_prompt` de la tabla `settings`
**And** SHALL usar ese valor como mensaje de sistema
**And** SHALL NOT usar ningún template hardcodeado en `OrchestratorConfig`
**And** el prompt SHALL contener instrucciones de Markdown, "Emojis" y formato rico
**And** si `system_prompt` está ausente o vacío, SHALL usar un fallback mínimo genérico y loguear un warning

#### Scenario: System prompt incluye personaje de mayordomo
**Given** la base de datos migrada
**When** se lee `settings.system_prompt`
**Then** contiene "asistente personal británico"
**And** contiene "usted"
**And** contiene "caballero"

#### Scenario: System prompt tiene modo conciso y expandido
**Given** la base de datos migrada
**When** se lee `settings.system_prompt`
**Then** contiene "Modo Conciso (Predeterminado)"
**And** contiene "Expandido"

#### Scenario: System prompt permite Markdown completo y emojis
**Given** la base de datos migrada
**When** se lee `settings.system_prompt`
**Then** contiene "Markdown"
**And** contiene "Emojis"

#### Scenario: process_message usa el prompt de la BD
**Given** un orquestador con `settings.system_prompt = "Prompt de prueba"`
**When** se llama `process_message()`
**Then** el mensaje de sistema enviado al LLM es "Prompt de prueba"

#### Scenario: process_message_stream usa el prompt de la BD
**Given** un orquestador con `settings.system_prompt = "Prompt de prueba"`
**When** se llama `process_message_stream()`
**Then** el mensaje de sistema enviado al LLM es "Prompt de prueba"

#### Scenario: Fallback mínimo si falta el prompt
**Given** un orquestador cuya tabla `settings` no tiene `system_prompt`
**When** se construye la petición al LLM
**Then** se usa un fallback mínimo genérico no vacío
**And** se loguea un warning indicando que falta el prompt

### Requirement: Tool execution errors do not break the ReAct loop

El orquestador SHALL convertir los errores de ejecución de herramientas en un `ToolResult { success: false }` y continuar el bucle ReAct.
**Given** el orquestador ejecuta un tool call
**When** `registry.execute()` retorna `Err(ToolError)` (e.g. timeout, parse error)
**Then** el error NO DEBE propagarse con `?` rompiendo el loop
**And** SE DEBE convertir en `ToolResult { success: false, message: error.to_string() }`
**And** el mensaje de error DEBE pasarse al LLM como tool result para que pueda responder
**And** el ReAct loop DEBE continuar normalmente

#### Scenario: Overpass timeout no rompe el orquestador
**Given** un orquestador con el tool `geo`
**When** el LLM invoca `geo.search_places`
**And** Overpass no responde (timeout de 30s)
**Then** `registry.execute()` retorna `Err(ToolError::ExecutionError("timeout"))`
**And** el orquestador NO propaga el error
**And** envía un `SSEEvent::ToolResult` con `success: false`
**And** pasa el mensaje de error al LLM como tool result
**And** el ReAct loop continúa

#### Scenario: Tool error tiene mensaje descriptivo para el LLM
**Given** un orquestador procesando un tool call
**When** el tool retorna error
**Then** el mensaje enviado al LLM es `format!("Error: {}", error)`
**And** incluye detalles del error (status code, raw body si aplica)

#### Scenario: Herramientas exitosas no se ven afectadas
**Given** un orquestador
**When** un tool call retorna `Ok(ToolResult { success: true, ... })`
**Then** el comportamiento NO cambia
**And** el tool result se pasa al LLM normalmente

### Requirement: Per-tool max retry limit of 3 in ReAct loop

El orquestador SHALL limitar a 3 las llamadas a una misma herramienta por bucle ReAct y continuar el bucle tras alcanzar el límite.
**Given** el orquestador ejecuta el ReAct loop
**When** un tool es invocado 3 veces o más en el mismo loop
**Then** NO DEBE ejecutarse el tool otra vez
**And** SE DEBE enviar un mensaje al LLM indicando que el tool no está disponible tras 3 intentos
**And** el ReAct loop DEBE continuar para que el LLM responda con alternativas

#### Scenario: Tool fails 3 times, 4th call is blocked
**Given** un orquestador con un tool que siempre falla
**When** el LLM llama al tool por 4ª vez en el mismo ReAct loop
**Then** el tool NO se ejecuta
**And** el LLM recibe el mensaje "Tool 'X' has been called 3 times. No more retries allowed."
**And** el ReAct loop continúa

#### Scenario: Successful tool calls don't count towards limit
**Given** un orquestador
**When** un tool se ejecuta con éxito 2 veces
**Then** el contador del tool aumenta a 2
**And** aún se puede llamar una 3ª vez
**And** el límite de 3 aplica tanto a fallos como a éxitos

#### Scenario: Different tools have independent counters
**Given** un orquestador
**When** el tool "geo" se ha llamado 3 veces y el tool "weather" 1 vez
**Then** "geo" está bloqueado
**And** "weather" aún puede ejecutarse

### Requirement: Orchestrator uses chat_stream for incremental response delivery

El orquestador SHALL usar `self.llm.chat_stream()` en la iteración final y emitir cada fragmento como `SSEEvent::Chunk`.

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

### Requirement: Tool error logging

El orquestador SHALL registrar a nivel `error!` los errores de herramientas con nombre, argumentos y mensaje completo.

All tool execution errors must be logged with maximum detail including
tool name, arguments, and the full error message (Display + Debug).

#### Scenario: ToolError from registry.execute() is logged with error!
When a tool returns `Err(ToolError)`, the orchestrator must log:
- `tool_name` — the tool that failed
- `tool_args` — the JSON arguments passed to the tool
- `error` — the error Display text
- `error_debug` — the error Debug representation

#### Scenario: ToolResult success=false is logged with error!
When a tool returns `ToolResult { success: false, ... }`, the orchestrator
must log the tool name, arguments, and error message.

#### Scenario: Tool retry limit reached is logged with warn!
When a tool has been called 3 times in the same ReAct loop, the orchestrator
must log a warning with the tool name and call count.

### Requirement: Done event tool_calls take precedence over stream events

El orquestador SHALL usar los `tool_calls` del evento Done (con argumentos completos) y caer en los del stream solo cuando el Done no los traiga.

#### Scenario: Done event tool_calls have full arguments
**Given** a Done event with `response.message.tool_calls = Some([...])` containing
full arguments from `StreamAccumulator.finalize()`
**When** the Done handler processes tool_calls
**Then** it uses the tool_calls from the Done response (with full arguments)
**And** not from the stream events (which have `arguments: Value::Null`)

#### Scenario: Done event without tool_calls falls back to stream
**Given** a Done event with `response.message.tool_calls = None`
**And** `tool_calls_from_stream` has tool calls
**When** the Done handler processes
**Then** it falls back to `tool_calls_from_stream`

### Requirement: Orchestrator records stats after each LLM call
The Orchestrator SHALL call `StatsRepo::record_request()` after each LLM call in both `process_message()` and `process_message_stream()`.

#### Scenario: process_message records stats
- **WHEN** `process_message("profile-1", "hello")` completes one iteration
- **THEN** there is exactly one row in `llm_requests` with status = "success"

#### Scenario: process_message_stream records stats
- **WHEN** `process_message_stream()` receives `StreamEvent::Done`
- **THEN** there is at least one row in `llm_requests` with tokens

#### Scenario: LLM error records stats with error status
- **WHEN** a mock LLM that always fails is used with `process_message()`
- **THEN** there is a row in `llm_requests` with status = "error" and non-empty error_message
