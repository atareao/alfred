# API Specification — Remove Multi-Conversation

### Requirement: Rutas REST simplificadas

**Given** la aplicación arrancada  
**When** se consultan las rutas disponibles  
**Then** las siguientes rutas NO existen (devuelven 404):
- `GET /api/conversations`
- `POST /api/conversations`
- `GET /api/conversations/main`
- `GET /api/conversations/:id`
- `PUT /api/conversations/:id`
- `DELETE /api/conversations/:id`
- `GET /api/conversations/:id/messages`
- `POST /api/conversations/:id/messages`
- `GET /api/conversations/:id/messages/:msg_id`
- `POST /api/conversations/:id/messages-stream`

**And** existen las siguientes rutas NUEVAS:
- `GET /api/messages` — lista mensajes (paginado, más recientes primero)
- `POST /api/messages` — crea un mensaje
- `GET /api/messages/:id` — obtiene un mensaje por ID
- `POST /api/chat/stream` — SSE stream para enviar mensaje al orquestador
- `GET /api/chat/init` — obtiene el estado inicial (primeros mensajes + settings)

#### Scenario: GET /api/messages devuelve mensajes sin conversation_id
**Given** la BD tiene mensajes  
**When** `GET /api/messages?limit=50`  
**Then** `200 OK`  
**And** el body es `{ "data": [...], "next_cursor": "..." }`  
**And** cada mensaje en `data` NO tiene campo `conversation_id`

#### Scenario: POST /api/messages crea sin conversation_id
**Given** la BD existe  
**When** `POST /api/messages` con `{ "role": "user", "content": "Hola" }`  
**Then** `201 Created`  
**And** el mensaje devuelto NO tiene campo `conversation_id`

#### Scenario: POST /api/chat/stream inicia stream SSE
**Given** el orquestador configurado  
**When** `POST /api/chat/stream` con `{ "content": "Hola" }`  
**Then** `200 OK` con `Content-Type: text/event-stream`  
**And** los eventos SSE NO incluyen `conversation_id`

#### Scenario: GET /api/chat/init devuelve estado inicial
**Given** la BD tiene mensajes y settings
**When** `GET /api/chat/init`
**Then** `200 OK`
**And** el body contiene `{ "messages": [...], "settings": {...} }`