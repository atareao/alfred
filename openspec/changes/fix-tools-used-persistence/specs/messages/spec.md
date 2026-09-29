# messages Handler Specification

## Change Delta

### Requirement: POST /api/messages SHALL accept optional tools_used field

**Given** el handler `create_message`  
**When** recibe un body con `tools_used: Option<String>`  
**Then** SHALL pasar ese valor a `MessagesRepo::create()`  
**And** SHALL persistirlo en la columna `tools_used` de la tabla `messages`

#### Scenario: Crear mensaje con tools_used via API
**Given** un body `{ "role": "assistant", "content": "Respuesta", "tools_used": "weather::get_weather" }`  
**When** se llama a `POST /api/messages`  
**Then** el mensaje creado SHALL tener `tools_used = Some("weather::get_weather")`

#### Scenario: Crear mensaje sin tools_used via API
**Given** un body `{ "role": "user", "content": "Hola" }`  
**When** se llama a `POST /api/messages`  
**Then** el mensaje creado SHALL tener `tools_used = None`

### Requirement: CreateMessage struct SHALL include optional tools_used field

**Given** el struct `CreateMessage` en `src/models/message.rs`  
**When** se define  
**Then** SHALL incluir `pub tools_used: Option<String>`

### Requirement: TypeScript CreateMessage interface SHALL include optional tools_used

**Given** la interfaz `CreateMessage` en `frontend/src/types/index.ts`  
**When** se define  
**Then** SHALL incluir `tools_used?: string`