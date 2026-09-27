# Message Display: Timestamp, Location & Date Separators

## ADDED Requirements

### Requirement: Message TypeScript interface SHALL include location

**Given** el tipo `Message` en `frontend/src/types/index.ts`  
**When** se renderiza un mensaje  
**Then** `Message` SHALL incluir `location?: string | null`

### Requirement: ChatView SHALL show date separators between message groups

**Given** una lista de mensajes ordenados cronológicamente  
**When** se renderizan en ChatView  
**Then** entre grupos de mensajes del mismo día calendario SHALL mostrar un `DateSeparator`

#### Scenario: Misma fecha → sin separador entre mensajes consecutivos
**Given** dos mensajes con `created_at` del mismo día (ej. "2026-09-27T10:00:00Z" y "2026-09-27T11:00:00Z")  
**When** se renderizan  
**Then** NO hay separador entre ellos

#### Scenario: Fecha diferente → separador entre grupos
**Given** un mensaje con fecha "2026-09-26T23:00:00Z" y otro con "2026-09-27T01:00:00Z"  
**When** se renderizan  
**Then** aparece un `DateSeparator` con formato legible entre ambos mensajes

#### Scenario: DateSeparator muestra "Hoy" para fecha actual
**Given** la fecha actual es 2026-09-27  
**Given** un mensaje con fecha "2026-09-27T10:00:00Z"  
**When** se renderiza  
**Then** el separador muestra "Hoy"

#### Scenario: DateSeparator muestra "Ayer" para día anterior
**Given** la fecha actual es 2026-09-27  
**Given** un mensaje con fecha "2026-09-26T10:00:00Z"  
**When** se renderiza  
**Then** el separador muestra "Ayer"

#### Scenario: DateSeparator muestra fecha completa para días más antiguos
**Given** un mensaje con fecha "2026-09-25T10:00:00Z"  
**When** se renderiza  
**Then** el separador muestra "25 sept 2026"

### Requirement: MessageBubble SHALL show timestamp inline and non-intrusive

**Given** un mensaje renderizado  
**When** se visualiza  
**Then** dentro de la burbuja, junto al role label, SHALL aparecer la hora en formato `HH:mm`

#### Scenario: Timestamp formatea created_at a hora local
**Given** `message.created_at = "2026-09-27T10:30:00Z"` (UTC)  
**When** se renderiza  
**Then** se muestra `10:30` (o la hora en timezone local del settings)

#### Scenario: Timestamp con location cuando existe
**Given** `message.created_at = "2026-09-27T10:30:00Z"` y `message.location = "Silla, Valencia, España"`  
**When** se renderiza  
**Then** se muestra `10:30 · 📍 Silla` (truncado a ciudad/pueblo)

#### Scenario: Timestamp sin location
**Given** `message.created_at = "2026-09-27T10:30:00Z"` y `message.location = null`  
**When** se renderiza  
**Then** se muestra solo `10:30`

#### Scenario: Mensaje streaming no muestra timestamp
**Given** mensaje con `id = "streaming"`  
**When** se renderiza  
**Then** NO se muestra timestamp ni location