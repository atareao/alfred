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

### Requirement: MessagesRepo::list_by_conversation SHALL use configurable page size

**Given** una conversación con mensajes en DB
**When** se llama `MessagesRepo::list_by_conversation(conn, conv_id, limit, cursor)`
**Then** el límite SHALL estar clampado entre 1 y 100
**And** el límite por defecto desde el handler SHALL venir del setting `message_page_size`

#### Scenario: Límite respeta clamp máximo
**Given** limit = 200
**When** se llama `list_by_conversation(conn, conv_id, 200, None)`
**Then** el clamp SHALL limitar a 100

#### Scenario: Límite respeta clamp mínimo
**Given** limit = 0
**When** se llama `list_by_conversation(conn, conv_id, 0, None)`
**Then** el clamp SHALL limitar a 1

### Requirement: SettingsRepo SHALL seed message_page_size default

**Given** una base de datos recién migrada
**When** se consulta `SettingsRepo::get(conn, "message_page_size")`
**Then** devuelve `Some("50")`

### Requirement: list_recent SHALL be removed

**Given** el código base actual
**When** se busca `list_recent` en `MessagesRepo`
**Then** la función SHALL haber sido eliminada
**And** sus tests SHALL haber sido eliminados

### Requirement: StatsRepo SHALL provide LLM usage aggregation queries

**Given** una tabla `llm_requests` con datos poblados
**When** se llama a `StatsRepo::summary(pool)`
**Then** devuelve un `StatsSummary` con:
- `total_calls: u64` — número total de llamadas
- `total_prompt_tokens: u64` — suma de prompt_tokens
- `total_completion_tokens: u64` — suma de completion_tokens
- `total_tokens: u64` — suma de total_tokens
- `total_cached_tokens: u64` — suma de cached_tokens
- `total_reasoning_tokens: u64` — suma de reasoning_tokens
- `total_cost: f64` — suma de cost
- `total_errors: u64` — llamadas con status != 'success'
- `avg_duration_ms: Option<f64>` — media de duration_ms (None si no hay datos)

#### Scenario: Summary con datos variados
**Given** 3 llamadas:
- éxito: prompt=100, completion=50, total=150, cached=10, reasoning=5, cost=0.01
- éxito: prompt=200, completion=100, total=300, cached=20, reasoning=15, cost=0.02
- error: prompt=0, completion=0, total=0, cached=0, reasoning=0, cost=0.0
**When** `StatsRepo::summary(pool)`
**Then** total_calls=3, total_prompt_tokens=300, total_completion_tokens=150, total_tokens=450, total_cached_tokens=30, total_reasoning_tokens=20, total_cost=0.03, total_errors=1

#### Scenario: Summary sin datos
**Given** tabla `llm_requests` vacía
**When** `StatsRepo::summary(pool)`
**Then** total_calls=0, total_cost=0.0, total_errors=0, avg_duration_ms=None

### Requirement: StatsRepo SHALL provide per-model breakdown

**Given** una tabla `llm_requests` con datos de múltiples modelos
**When** se llama a `StatsRepo::by_model(pool)`
**Then** devuelve `Vec<ModelStats>` con un elemento por modelo, cada uno con:
- `model: String`
- `calls: u64`, `total_tokens: u64`, `total_cost: f64`, `avg_duration_ms: Option<f64>`, `total_cached_tokens: u64`, `total_reasoning_tokens: u64`

#### Scenario: Dos modelos con datos
**Given** 2 llamadas a "gpt-4o" y 1 a "claude-3"
**When** `StatsRepo::by_model(pool)`
**Then** devuelve 2 filas ordenadas por coste descendente

### Requirement: StatsRepo SHALL provide daily time series

**Given** una tabla `llm_requests` con datos de varios días
**When** se llama a `StatsRepo::by_day(pool, days)`
**Then** devuelve `Vec<DayStats>` con un elemento por día, cada uno con:
- `date: String` (formato YYYY-MM-DD)
- `calls: u64`, `total_tokens: u64`, `total_cost: f64`, `total_cached_tokens: u64`, `total_reasoning_tokens: u64`

#### Scenario: Datos de 7 días
**Given** llamadas distribuidas en 7 días
**When** `StatsRepo::by_day(pool, 30)`
**Then** devuelve 7 filas ordenadas por fecha ascendente

### Requirement: StatsRepo SHALL provide tool call frequency

**Given** una tabla `llm_requests` con tool_calls poblados
**When** se llama a `StatsRepo::tools_summary(pool)`
**Then** devuelve `Vec<ToolStats>` con cada tool y su frecuencia de uso

#### Scenario: Tools más usadas
**Given** 5 llamadas: 3 con tool_calls '["get_weather"]', 2 con '["search_web"]'
**When** `StatsRepo::tools_summary(pool)`
**Then** get_weather: 3, search_web: 2

### Requirement: StatsRepo SHALL provide database table sizes

**Given** una base de datos con tablas pobladas
**When** se llama a `StatsRepo::db_sizes(pool)`
**Then** devuelve `Vec<TableSize>` con nombre de tabla y row count para:
messages, profiles, memories, events, tasks, notes, contacts, reminders, meal_plans, shopping_list, habits, habit_logs, tools

#### Scenario: Tablas con datos
**Given** 10 messages, 2 profiles, 5 memories
**When** `StatsRepo::db_sizes(pool)`
**Then** messages=10, profiles=2, memories=5, resto=0

### Requirement: StatsRepo SHALL provide CSV export with all OpenRouter fields

**Given** una tabla `llm_requests` con datos
**When** se llama a `StatsRepo::export_csv(pool)`
**Then** devuelve un String con formato CSV con cabeceras:
`id,model,provider,prompt_tokens,completion_tokens,total_tokens,cached_tokens,reasoning_tokens,cost,is_byok,duration_ms,cache_hit,status,error_message,tool_calls,created_at`

#### Scenario: Export CSV con datos
**Given** 2 llamadas en llm_requests
**When** `StatsRepo::export_csv(pool)`
**Then** el CSV tiene 1 línea de cabecera + 2 líneas de datos
**And** cada línea incluye cost, cached_tokens, reasoning_tokens

### Requirement: StatsRepo SHALL purge data older than retention period

**Given** una tabla `llm_requests` con datos de 60 días
**When** se llama a `StatsRepo::purge_old(pool, 30)`
**Then** borra todos los registros con created_at anterior a hace 30 días
**And** devuelve el número de registros eliminados

#### Scenario: Purga con datos mixtos
**Given** 10 registros de hace 45 días y 10 de hace 15 días
**When** `StatsRepo::purge_old(pool, 30)`
**Then** elimina 10 registros (los de 45 días)
**And** devuelve 10
