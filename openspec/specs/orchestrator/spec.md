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
