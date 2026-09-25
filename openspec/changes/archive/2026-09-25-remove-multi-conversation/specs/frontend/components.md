# Frontend Specification — Remove Multi-Conversation

### Requirement: Layout de una sola columna

**Given** la aplicación cargada  
**When** se renderiza `AppLayout`  
**Then** NO existe sidebar de conversaciones  
**And** el layout ocupa todo el ancho de la ventana  
**And** no hay menú de navegación lateral

#### Scenario: Sin sidebar de conversaciones
**Given** el usuario abre Alfred  
**When** se renderiza la página  
**Then** no hay elemento con clase/rol de sidebar  
**And** no hay lista de conversaciones

### Requirement: ChatView sin conversationId

**Given** el hook `useMainChat`  
**When** se inicializa  
**Then** no necesita `conversationId`  
**And** llama a `GET /api/chat/init` para obtener estado inicial  
**And** `sendMessage` no necesita `conversationId`  
**And** llama a `POST /api/chat/stream` directamente

#### Scenario: useMainChat no depende de conversationId
**Given** el hook useMainChat  
**When** se monta el componente  
**Then** `loading` es `true`  
**And** tras recibir respuesta de `/api/chat/init`, `loading` es `false`  
**And** `messages` contiene los mensajes iniciales

### Requirement: Tipos sin conversation_id

**Given** el archivo `types/index.ts`  
**When** se define `Message`  
**Then** NO tiene campo `conversation_id`  
**And** NO existe el tipo `Conversation`

#### Scenario: Message sin conversation_id
**Given** una respuesta del API  
**When** se parsea como `Message`  
**Then** no hay campo `conversation_id`  
**And** los campos son: `id`, `role`, `content`, `created_at`

### Requirement: Eliminar hooks huérfanos

**Given** el directorio `frontend/src/hooks/`  
**When** se listan los archivos  
**Then** NO existe `useEphemeralChat.ts`  
**And** NO existe `useBrowserContext.ts` (integrado en useMainChat si es necesario)

#### Scenario: Hooks eliminados
**Given** el proyecto  
**When** se compila con `npx tsc --noEmit`  
**Then** no hay errores de imports faltantes por `useEphemeralChat` ni `useBrowserContext`