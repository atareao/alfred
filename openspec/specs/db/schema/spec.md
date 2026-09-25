# db/schema Specification

## Purpose
TBD - created by archiving change settings-token-window. Update Purpose after archive.

## Requirements

### Requirement: Tabla settings con valores por defecto
**Given** una base de datos recién migrada  
**When** se ejecuta `run_migrations()`  
**Then** existe la tabla `settings` con columnas `key`, `value`, `updated_at`  
**And** contiene los valores por defecto `max_window_tokens=10000` y `system_prompt`

#### Scenario: Migración crea tabla settings
**Given** una base de datos vacía  
**When** se ejecuta `run_migrations()`  
**Then** la tabla `settings` existe  
**And** `SELECT value FROM settings WHERE key='max_window_tokens'` devuelve `10000`  
**And** `SELECT value FROM settings WHERE key='system_prompt'` devuelve el prompt por defecto

### Requirement: SettingsRepo get/set/get_all
**Given** un SettingsRepo sobre una conexión  
**When** se llama `get(conn, "max_window_tokens")`  
**Then** devuelve `Some("10000")` si existe, `None` si no  
**When** se llama `set(conn, "key", "value")`  
**Then** inserta o actualiza el valor  
**When** se llama `get_all(conn)`  
**Then** devuelve un HashMap con todas las claves y valores

#### Scenario: get devuelve valor existente
**Given** settings con `max_window_tokens=10000`  
**When** `get(conn, "max_window_tokens")`  
**Then** devuelve `Some("10000")`

#### Scenario: set inserta nuevo valor
**Given** settings sin clave `foo`  
**When** `set(conn, "foo", "bar")`  
**Then** `get(conn, "foo")` devuelve `Some("bar")`

#### Scenario: set actualiza valor existente
**Given** settings con `foo=bar`  
**When** `set(conn, "foo", "baz")`  
**Then** `get(conn, "foo")` devuelve `Some("baz")`

### Requirement: Tabla messages con todas las columnas desde CREATE TABLE

**Given** una base de datos recién migrada  
**When** se ejecuta `run_migrations()`  
**Then** existe la tabla `messages` con todas las columnas en un solo CREATE TABLE:
`id`, `conversation_id`, `role`, `content`, `tool_calls`, `tool_results`,
`created_at`, `tokens_count`, `collapsed_content`, `collapsed_tokens_count`,
`is_indexed`, `summary_ref`

**And** no existe la migración `20260925000002_message_enrichment.sql` en disco

#### Scenario: Migración #01 incluye columnas de enrichment
**Given** una base de datos vacía  
**When** se ejecuta `run_migrations()`  
**Then** `PRAGMA table_info(messages)` contiene `tokens_count`, `collapsed_content`,
`collapsed_tokens_count`, `is_indexed`, `summary_ref`

#### Scenario: Migración #02 no existe en disco
**Given** el directorio `migrations/`  
**When** se listan los archivos  
**Then** no existe `20260925000002_message_enrichment.sql`
