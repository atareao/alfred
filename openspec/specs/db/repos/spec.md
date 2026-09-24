# db/repos Specification

## Purpose
TBD - created by archiving change sql-window-history. Update Purpose after archive.

## Requirements

### Requirement: MessagesRepo::list_by_token_budget SHALL select messages by token budget

**Given** una conversación con mensajes almacenados en DB  
**When** se llama `MessagesRepo::list_by_token_budget(conn, conversation_id, max_tokens)`  
**Then** devuelve los mensajes más recientes cuya suma acumulada de `tokens_count`
(o `collapsed_tokens_count` si existe) no supere `max_tokens`  
**And** los mensajes se devuelven en orden cronológico ascendente

#### Scenario: Selecciona mensajes dentro del presupuesto
**Given** una conversación con 3 mensajes de 1000, 2000 y 1000 tokens respectivamente  
**When** `list_by_token_budget(conn, conv_id, 3500)`  
**Then** devuelve los 2 mensajes más recientes (2000 + 1000 = 3000 ≤ 3500)  
**And** el mensaje más antiguo (1000) NO se incluye

#### Scenario: Presupuesto suficiente para todos los mensajes
**Given** una conversación con 2 mensajes de 500 tokens cada uno  
**When** `list_by_token_budget(conn, conv_id, 2000)`  
**Then** devuelve ambos mensajes

#### Scenario: Usa collapsed_tokens_count cuando existe
**Given** un mensaje con `tokens_count: 3000` y `collapsed_tokens_count: 200`  
**When** `list_by_token_budget(conn, conv_id, 500)`  
**Then** el mensaje colapsado se incluye (200 ≤ 500)  
**And** el contenido devuelto es `collapsed_content`

#### Scenario: Presupuesto cero devuelve lista vacía
**Given** una conversación con mensajes  
**When** `list_by_token_budget(conn, conv_id, 0)`  
**Then** devuelve una lista vacía

#### Scenario: Conversación sin mensajes
**Given** una conversación sin mensajes  
**When** `list_by_token_budget(conn, conv_id, 10000)`  
**Then** devuelve una lista vacía
