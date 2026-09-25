# Orchestrator Specification — Remove Multi-Conversation

### Requirement: Orchestrator sin conversation_id

**Given** el struct `Orchestrator`  
**When** se llama `process_message_stream()`  
**Then** NO recibe parámetro `conversation_id`  
**And** `list_by_token_budget()` se llama sin filtrar por `conversation_id`  
**And** `MessagesRepo::create()` se llama sin `conversation_id`

#### Scenario: process_message_stream sin conversation_id
**Given** un orchestrator configurado  
**When** el usuario envía un mensaje via SSE  
**Then** el mensaje se persiste en `messages` SIN `conversation_id`  
**And** el historial se carga de TODOS los mensajes (sin filtro)

### Requirement: ContextBuilder sin conversation_id

**Given** el struct `ContextBuilder`  
**When** se llama `build()`  
**Then** no recibe ni usa `conversation_id`

#### Scenario: build sin conversation_id
**Given** un ContextBuilder  
**When** se construye el contexto  
**Then** solo usa `profile_id` y `user_message`  
**And** no referencia ninguna conversación