# db/schema Specification — Delta: unify-message-migrations

## ADDED Requirements

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