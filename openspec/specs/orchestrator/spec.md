# orchestrator Specification

## Purpose
TBD - created by archiving change sql-window-history. Update Purpose after archive.

## Requirements

### Requirement: Orchestrator SHALL use list_by_token_budget instead of SessionWindow

**Given** un orquestador procesando un mensaje  
**When** se construye el array de mensajes para el LLM  
**Then** el historial se carga mediante `MessagesRepo::list_by_token_budget()`  
**And** NO se usa `SessionWindow::get_window()`  
**And** el setting `max_window_tokens` controla el presupuesto de tokens

#### Scenario: Historial se carga desde DB con token budget
**Given** un orquestador con `max_window_tokens = 4000`  
**When** se procesa un mensaje  
**Then** el historial contiene solo los mensajes que caben en 4000 tokens  
**And** no hay referencia a `SessionWindow` en el código del orquestador

#### Scenario: Sin SessionWindow en el estado del orquestador
**Given** un orquestador  
**Then** su constructor NO recibe `Arc<Mutex<SessionWindow>>`  
**And** no tiene campo `session_window`

### Requirement: Orchestrator SHALL store tools_used as metadata, not in content

**Given** el orquestador procesando un mensaje con herramientas  
**When** se completa el ReAct loop y se persiste el mensaje assistant  
**Then** el contenido del mensaje NO SHALL incluir footer de herramientas  
**And** las herramientas SHALL almacenarse en el campo `tools_used`  
**And** el formato SHALL ser `"(N) tool::operation, tool::operation"` donde N es el contador si > 1

#### Scenario: Herramientas únicas sin operación
**Given** herramientas usadas: `["weather", "calendar"]`  
**When** se construye `tools_used`  
**Then** el resultado SHALL ser `"weather, calendar"`

#### Scenario: Misma herramienta con diferentes operaciones
**Given** herramientas usadas: `["calendar::get_events", "calendar::create_event", "weather::get_weather"]`  
**When** se construye `tools_used`  
**Then** el resultado SHALL ser `"calendar::get_events, calendar::create_event, weather::get_weather"`

#### Scenario: Misma herramienta y operación repetida
**Given** herramientas usadas: `["calendar::get_events", "calendar::get_events", "calendar::get_events"]`  
**When** se construye `tools_used`  
**Then** el resultado SHALL ser `"(3) calendar::get_events"`

#### Scenario: Mezcla de únicas y repetidas
**Given** herramientas usadas: `["calendar::get_events", "calendar::get_events", "weather::get_weather", "calendar::create_event"]`  
**When** se construye `tools_used`  
**Then** el resultado SHALL ser `"(2) calendar::get_events, weather::get_weather, calendar::create_event"`
