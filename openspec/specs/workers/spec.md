# workers Specification

## Purpose
TBD - created by archiving change collapse-worker. Update Purpose after archive.

## Requirements

### Requirement: CollapseWorker SHALL process messages in background
**Given** a message ID is sent through the collapse channel  
**When** the `CollapseWorker` receives it  
**Then** it SHALL read the message from the database  
**And** it SHALL call the LLM with the collapse prompt  
**And** it SHALL update `collapsed_content` and `collapsed_tokens_count` in the database

#### Scenario: Worker collapses a long message
**Given** a message with id "msg-1" and content of 8000 chars exists in DB  
**When** "msg-1" is sent through the collapse channel  
**Then** the worker SHALL read the message  
**And** SHALL call `llm_provider.chat()` with the collapse prompt  
**And** SHALL update `collapsed_content` to the LLM response  
**And** SHALL update `collapsed_tokens_count` to `estimate_tokens(response)`

### Requirement: Collapse prompt SHALL be configurable via settings
**Given** the `settings` table has a `collapse_prompt` key  
**When** the worker starts  
**Then** it SHALL read `collapse_prompt` from settings  
**And** SHALL use it as the system prompt for the LLM call  
**And** SHALL fall back to a default prompt if not set

#### Scenario: Default collapse prompt
**Given** no `collapse_prompt` in settings  
**When** the worker processes a message  
**Then** it SHALL use the default prompt: "Resume el siguiente texto manteniendo la información clave, los datos importantes y el contexto necesario. Sé conciso."

#### Scenario: Custom collapse prompt
**Given** `collapse_prompt` = "Summarize in 3 bullet points" in settings  
**When** the worker processes a message  
**Then** it SHALL use "Summarize in 3 bullet points" as the system prompt

### Requirement: Collapse model SHALL be configurable via env var
**Given** `COLLAPSE_MODEL` env var  
**When** `Config::from_env()` is called  
**Then** `collapse_model` SHALL take the env var value  
**And** SHALL default to `mistralai/mistral-small`

#### Scenario: Default collapse model
**Given** no `COLLAPSE_MODEL` env var  
**When** `Config::from_env()` is called  
**Then** `collapse_model` SHALL be `"mistralai/mistral-small"`

#### Scenario: Custom collapse model
**Given** `COLLAPSE_MODEL` = `"google/gemini-2.0-flash-lite"`  
**When** `Config::from_env()` is called  
**Then** `collapse_model` SHALL be `"google/gemini-2.0-flash-lite"`

### Requirement: Handler SHALL wire collapse callback on message creation
**Given** a message with `tokens_count >= collapse_threshold_tokens`  
**When** `create_message` handler is called  
**Then** it SHALL send the message ID through the collapse channel

#### Scenario: Short message does not trigger collapse
**Given** a message with 100 chars  
**When** created via the API  
**Then** the collapse channel SHALL NOT receive the message ID

#### Scenario: Long message triggers collapse
**Given** a message with 8000 chars  
**When** created via the API  
**Then** the collapse channel SHALL receive the message ID

### Requirement: WorkerPool SHALL include CollapseWorker
**Given** a `WorkerPool::start()` call  
**Then** it SHALL spawn a `CollapseWorker`  
**And** `pool.collapse` SHALL be `Some`

#### Scenario: CollapseWorker starts and shuts down
**Given** a started WorkerPool
**When** `pool.shutdown()` is called
**Then** the collapse worker SHALL be aborted

### Requirement: CollapseWorker SHALL accept model parameter
**Given** `CollapseWorker::start()`
**When** se invoca
**Then** SHALL aceptar un parámetro `model: String`
**And** SHALL usar ese modelo en `ChatRequest.model` en lugar del hardcodeado `"mistralai/mistral-small"`

#### Scenario: Worker usa el modelo pasado como parámetro
**Given** `model = "google/gemini-2.0-flash-lite"`
**When** se inicia el worker
**Then** `ChatRequest.model` SHALL ser `"google/gemini-2.0-flash-lite"`

### Requirement: WorkerPool SHALL use real CollapseWorker
**Given** `WorkerPool::start()`
**When** se inicia el pool
**Then** SHALL usar `CollapseWorker::start()` con el modelo de `config.collapse_model`
**And** SHALL leer `collapse_prompt` de settings y pasarlo al worker
**And** el placeholder actual (que solo logea) SHALL ser eliminado

#### Scenario: CollapseWorker recibe mensajes del canal
**Given** un WorkerPool iniciado con CollapseWorker real
**When** un mensaje largo se crea y supera el threshold
**Then** el worker SHALL recibir el message_id por el canal mpsc
**And** SHALL procesarlo (LLM + update DB)

### Requirement: Agent SHALL wire collapse callback
**Given** el orquestador con acceso a `collapse_tx`
**When** persiste mensajes de usuario y asistente
**Then** SHALL pasar `collapse_tx` como `on_collapse_needed` en `MessagesRepo::create()`

#### Scenario: Mensaje largo del usuario dispara collapse
**Given** un mensaje de usuario con 8000 chars
**When** el orquestador lo persiste
**Then** el message_id SHALL enviarse por el canal de collapse

#### Scenario: Mensaje largo del asistente dispara collapse
**Given** un mensaje de asistente con 8000 chars
**When** el orquestador lo persiste
**Then** el message_id SHALL enviarse por el canal de collapse
