# Settings: SessionWindow token-based + Orchestrator

## ADDED Requirements

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

### Requirement: Orchestrator lee max_window_tokens y system_prompt de settings en DB
**Given** un orchestrator procesando un mensaje  
**When** se inicia el ReAct loop  
**Then** lee `max_window_tokens` y `system_prompt` de la tabla settings  
**And** usa `system_prompt` en lugar del template por defecto si existe  
**And** sobreescribe `config.max_window_tokens` si viene de settings

#### Scenario: system_prompt de settings sobreescribe al template
**Given** settings con `system_prompt=Instrucciones personalizadas`  
**When** se procesa un mensaje  
**Then** el system prompt enviado al LLM es `Instrucciones personalizadas`

#### Scenario: sin system_prompt en settings usa el template
**Given** settings sin clave `system_prompt`  
**When** se procesa un mensaje  
**Then** se usa el `system_prompt_template` de OrchestratorConfig