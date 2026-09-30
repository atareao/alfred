# orchestrator Specification

## Purpose
Orquestador de Valet: carga del historial de conversación por presupuesto de tokens y almacenamiento de las herramientas usadas como metadatos del mensaje en lugar de incrustarlas en el contenido.

## Requirements

### Requirement: Orchestrator SHALL use list_by_token_budget instead of SessionWindow

El orquestador SHALL cargar el historial con `MessagesRepo::list_by_token_budget()` y SHALL NOT usar `SessionWindow`.

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

### Requirement: ContextBuilder SHALL be wired with the database pool and embedding provider

En producción, `ContextBuilder` SHALL construirse con `pool: Some(...)` y, cuando los embeddings estén configurados, `provider: Some(...)`, además de `rag_budget_tokens` leído de `RAG_BUDGET_TOKENS`.

**Given** `AppState::new_with_orchestrator()`  
**When** se construye el `ContextBuilder`  
**Then** `pool` SHALL ser `Some(...)`  
**And** `provider` SHALL ser `Some(...)` cuando `EMBEDDING_PROVIDER` y `EMBEDDING_MODEL` estén definidos  
**And** `rag_budget_tokens` SHALL leerse de `RAG_BUDGET_TOKENS`

#### Scenario: Builder de producción tiene pool
**Given** el arranque de producción  
**When** se inspecciona el `ContextBuilder`  
**Then** `pool` es `Some`

#### Scenario: Provider presente cuando hay configuración
**Given** `EMBEDDING_PROVIDER` y `EMBEDDING_MODEL` definidos  
**When** se construye el `ContextBuilder`  
**Then** `provider` es `Some`

#### Scenario: Provider ausente sin configuración
**Given** `EMBEDDING_PROVIDER` sin definir  
**When** se construye el `ContextBuilder`  
**Then** `provider` es `None`

### Requirement: RAG SHALL NOT inject placeholder memories

`ContextBuilder::build_rag_memories` SHALL devolver `Vec::new()` cuando falte el pool, falte el provider o falle la búsqueda, logueando un warning. SHALL NOT devolver memorias hardcodeadas.

**Given** un `ContextBuilder` sin pool o sin provider, o una búsqueda vectorial que falla  
**When** se construye el contexto con estrategia `RAG`  
**Then** `rag_memories` SHALL ser vacío  
**And** SHALL NOT contener valores hardcodeados como `"memory1"` o `"memory2"`  
**And** SHALL loguearse un warning

#### Scenario: Sin pool devuelve vacío
**Given** un `ContextBuilder` con `pool: None`  
**When** se construye el contexto `RAG`  
**Then** `rag_memories` es vacío

#### Scenario: Sin provider devuelve vacío
**Given** un `ContextBuilder` con `pool: Some` y `provider: None`  
**When** se construye el contexto `RAG`  
**Then** `rag_memories` es vacío

#### Scenario: Error de búsqueda devuelve vacío con warning
**Given** un `ContextBuilder` con pool y provider, y una búsqueda que falla  
**When** se construye el contexto `RAG`  
**Then** `rag_memories` es vacío  
**And** se loguea un warning

#### Scenario: Búsqueda real devuelve memorias formateadas
**Given** un `ContextBuilder` con pool y provider, y una fila en `memory` + `vec_memory`  
**When** se construye el contexto `RAG`  
**Then** `rag_memories` contiene la memoria formateada como `[{tags}] {content}`
