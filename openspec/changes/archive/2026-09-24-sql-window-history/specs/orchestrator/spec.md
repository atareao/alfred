# Orchestrator: SQL Window History

## ADDED Requirements

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

## REMOVED Requirements

### Requirement: SessionWindow usa max_window_tokens en lugar de max_window

**Given** una SessionWindow con `max_window_tokens: 10000`  
**When** se añaden mensajes  
**Then** se mantienen mientras la suma de tokens estimados no supere `max_window_tokens`  
**And** al superarlo, se eliminan los más antiguos hasta estar por debajo del threshold

#### Scenario: Sesión dentro del límite de tokens
**Given** una SessionWindow con `max_window_tokens: 10000`  
**When** se añaden mensajes con un total estimado de 5000 tokens  
**Then** no se elimina ningún mensaje

#### Scenario: Sesión excede el límite de tokens
**Given** una SessionWindow con `max_window_tokens: 1000`  
**When** se añaden mensajes hasta superar los 1000 tokens estimados  
**Then** se eliminan los mensajes más antiguos hasta estar por debajo del threshold

### Requirement: estimate_tokens(text) estima tokens para texto en español

**Given** un texto  
**When** se llama `estimate_tokens(text)`  
**Then** devuelve un número aproximado de tokens (aprox: chars/2 para español)

#### Scenario: estimate_tokens con texto corto
**Given** texto "Hola"  
**When** estimate_tokens  
**Then** devuelve ~2 (4 chars / 2 = 2)

#### Scenario: estimate_tokens con texto vacío
**Given** texto ""  
**When** estimate_tokens  
**Then** devuelve 0