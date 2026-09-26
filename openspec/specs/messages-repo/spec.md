# messages-repo Specification

## Purpose
Gestión del repositorio de mensajes de conversación. Creado tras archivar fix-load-messages.

## Requirements

### Requirement: list_all devuelve los mensajes más recientes
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