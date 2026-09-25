# orchestrator/agent Specification

## Purpose
TBD - created by archiving change fix-system-prompt-markdown. Update Purpose after archive.

## Requirements

### Requirement: System prompt template uses Markdown and emojis
**Given** la configuración por defecto del orquestador  
**When** se carga `OrchestratorConfig::default()`  
**Then** el `system_prompt_template` contiene instrucciones de Markdown  
**And** contiene "Emojis" y formato rico

#### Scenario: System prompt incluye personaje de mayordomo
**Given** la configuración por defecto  
**When** se accede a `system_prompt_template`  
**Then** contiene "mayordomo británico"  
**And** contiene **"usted"**  
**And** contiene "caballero"

#### Scenario: System prompt tiene modo conciso y expandido
**Given** la configuración por defecto  
**When** se accede a `system_prompt_template`  
**Then** contiene "Modo por defecto: conciso"  
**And** contiene "expandido"

#### Scenario: System prompt permite Markdown completo y emojis
**Given** la configuración por defecto  
**When** se accede a `system_prompt_template`  
**Then** contiene "Markdown"  
**And** contiene "emojis" o "Emojis"

### Requirement: Tool execution errors do not break the ReAct loop
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
