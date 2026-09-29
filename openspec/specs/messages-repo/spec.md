# messages-repo Specification

## Purpose
Gestión del repositorio de mensajes de conversación. Creado tras archivar fix-load-messages.

## Requirements

### Requirement: list_all devuelve los mensajes más recientes

`list_all` SHALL devolver como máximo `limit` mensajes, empezando por los más recientes y en orden cronológico ascendente.
**Given** la base de datos tiene mensajes  
**When** se llama `list_all(pool, limit, None)`  
**Then** devuelve como máximo `limit` mensajes, empezando por los más recientes, en orden cronológico ascendente  
**And** si hay más mensajes, `next_cursor` apunta al más antiguo del lote devuelto

#### Scenario: Menos de `limit` mensajes en DB
**Given** una DB con 3 mensajes (m1, m2, m3 en orden cronológico ascendente)  
**When** se llama `list_all(pool, 50, None)`  
**Then** devuelve `([m1, m2, m3], None)` en orden ascendente

#### Scenario: Más de `limit` mensajes en DB
**Given** una DB con 60 mensajes  
**When** se llama `list_all(pool, 50, None)`  
**Then** devuelve los 50 más recientes en orden ascendente  
**And** `next_cursor` no es None

### Requirement: Paginación hacia atrás con cursor

`list_all` SHALL soportar paginación hacia atrás mediante un cursor, devolviendo los mensajes anteriores y un nuevo cursor o `None`.
**Given** se ha cargado una página de mensajes  
**When** se llama `list_all(pool, limit, Some(cursor))` con cursor = timestamp del más antiguo  
**Then** devuelve los mensajes anteriores al cursor, en orden cronológico ascendente  
**And** si no hay más mensajes, `next_cursor` es None

#### Scenario: Dos páginas
**Given** una DB con 55 mensajes  
**When** página 1: `list_all(pool, 50, None)` → 50 mensajes, cursor no nulo  
**And** página 2: `list_all(pool, 50, Some(cursor))` → 5 mensajes  
**Then** todos los 55 mensajes se han devuelto entre ambas páginas

#### Scenario: DB vacía
**Given** una DB sin mensajes  
**When** se llama `list_all(pool, 50, None)`  
**Then** devuelve `([], None)`

### Requirement: MessagesRepo::create SHALL accept tools_used parameter

**Given** `MessagesRepo::create()`  
**When** se llama con `tools_used: Option<&str>`  
**Then** el valor SHALL almacenarse en la columna `tools_used` de la tabla `messages`  
**And** todas las queries SELECT SHALL incluir la columna `tools_used`

#### Scenario: Crear mensaje con tools_used
**Given** un pool de base de datos  
**When** `MessagesRepo::create(pool, "assistant", "Respuesta", None, None, None, Some("weather::get_weather"), 2000, None)`  
**Then** el mensaje devuelto SHALL tener `tools_used = Some("weather::get_weather")`

#### Scenario: Crear mensaje sin tools_used
**Given** un pool de base de datos  
**When** `MessagesRepo::create(pool, "user", "Hola", None, None, None, None, 2000, None)`  
**Then** el mensaje devuelto SHALL tener `tools_used = None`