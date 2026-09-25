# db/schema Specification — Remove Multi-Conversation

### Requirement: Esquema unificado sin tabla conversations

**Given** una base de datos recién inicializada  
**When** se ejecuta `run_migrations()`  
**Then** existe la tabla `messages` con columnas:
`id TEXT PRIMARY KEY`, `role TEXT NOT NULL`, `content TEXT NOT NULL`,
`tool_calls TEXT`, `tool_results TEXT`, `tokens_count INTEGER NOT NULL DEFAULT 0`,
`collapsed_content TEXT`, `collapsed_tokens_count INTEGER NOT NULL DEFAULT 0`,
`is_indexed INTEGER NOT NULL DEFAULT 0`, `summary_ref TEXT`,
`created_at TEXT NOT NULL DEFAULT (datetime('now'))`

**And** NO existe la tabla `conversations`  
**And** la tabla `messages` NO tiene columna `conversation_id`  
**And** existen las tablas `profiles`, `memories`, `settings`, `tools`,
`meal_plans`, `shopping_list`, `habits`, `habit_logs`, `events`,
`notes`, `contacts`, `reminders`, `tasks`, `tool_permissions`

#### Scenario: Migración única no crea conversations
**Given** una base de datos vacía  
**When** se ejecuta `run_migrations()`  
**Then** `SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='conversations'` = 0

#### Scenario: messages no tiene conversation_id
**Given** una base de datos migrada  
**When** se consulta `PRAGMA table_info(messages)`  
**Then** ninguna fila tiene `name = 'conversation_id'`

### Requirement: Migraciones reemplazadas por schema único

**Given** el directorio `migrations/`  
**When** se listan los archivos  
**Then** solo existe `20260925000001_initial.sql` (o archivo de schema único)  
**And** NO existen archivos de migración individuales para `core_tables`, `embedding_tables`, `fts5_tables`, `tools_tables`, `meal_plans`, `fts_triggers`

#### Scenario: Un solo archivo de migración
**Given** el directorio `migrations/`  
**When** se listan los archivos `.sql`  
**Then** hay exactamente 1 archivo de migración  
**And** ese archivo contiene el CREATE TABLE de `messages` SIN `conversation_id`