## ADDED Requirements

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