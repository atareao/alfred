# stream-route Specification

## Purpose
Rutas SSE para streaming de chat. Contiene el endpoint `POST /api/chat/stream`.

## Requirements

### Requirement: profile_id must resolve from DB, not be hardcoded
**Given** una base de datos sin perfil con id="profile-1"  
**When** se llama a `POST /api/chat/stream`  
**Then** el profile_id inyectado en los tool calls debe ser un ID real existente en la tabla `profiles`

#### Scenario: Hardcoded "profile-1" no existe en producción
**Given** una base de datos recién migrada (sin seed)  
**When** `ProfilesRepo::get_or_create(pool)` devuelve un profile con id UUID  
**Then** ese UUID debe ser el profile_id que se pase al orquestador  
**And** no debe usarse `"profile-1"` como string hardcodeado

#### Scenario: El handler resuelve el profile correctamente
**Given** un `AppState` con base de datos y orquestador  
**When** se envía una petición `POST /api/chat/stream`  
**Then** el stream debe funcionar sin errores de FK  
**And** debe usar el ID devuelto por `ProfilesRepo::get_or_create()`