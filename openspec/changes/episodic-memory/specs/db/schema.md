# db/schema Specification — Episodic Memory

## Requirements

### Requirement: New migration SHALL create memory table

**Given** una base de datos vacía  
**When** se ejecuta `run_migrations()`  
**Then** existe la tabla `memory` con las columnas:

```sql
CREATE TABLE IF NOT EXISTS memory (
    id           TEXT PRIMARY KEY,
    content      TEXT NOT NULL,
    tokens_count INTEGER NOT NULL DEFAULT 0,
    created_at   TEXT NOT NULL DEFAULT (datetime('now')),
    metadata     TEXT DEFAULT '{}'
);
```

#### Scenario: memory table structure
**Given** una base de datos vacía  
**When** se ejecutan las migraciones  
**Then** `PRAGMA table_info(memory)` contiene: id (TEXT PK), content (TEXT NOT NULL), tokens_count (INTEGER NOT NULL DEFAULT 0), created_at (TEXT), metadata (TEXT DEFAULT '{}')

#### Scenario: memory table is idempotent
**Given** la tabla memory ya existe  
**When** se ejecuta la migración de nuevo  
**Then** no hay error

### Requirement: New migration SHALL create vec_memory virtual table

**Given** una base de datos vacía  
**When** se ejecuta `run_migrations()`  
**Then** existe la tabla virtual `vec_memory`:

```sql
CREATE VIRTUAL TABLE IF NOT EXISTS vec_memory USING vec0(
    embedding float[1536] distance_metric=cosine
);
```

**And** `rowid` de `vec_memory` se vincula con `memory.id` (coincide: memory.id UUID → vec_memory.rowid es el mismo UUID como entero, o se usa mapping explícito).

#### Scenario: vec_memory table exists
**Given** migraciones ejecutadas  
**When** `SELECT name FROM sqlite_master WHERE type='table' AND name='vec_memory'`  
**Then** devuelve 'vec_memory'

### Requirement: Migration SHALL add FK from messages.summary_ref to memory.id

**Given** la tabla messages existe  
**When** se ejecuta la migración  
**Then** `summary_ref` en messages tiene FOREIGN KEY REFERENCES memory(id) ON DELETE SET NULL

#### Scenario: summary_ref FK exists
**Given** migraciones ejecutadas  
**When** se consulta `PRAGMA foreign_key_list(messages)`  
**Then** existe una FK donde `from` = "summary_ref" y `table` = "memory"

### Requirement: Migration SHALL create indexes on messages

**Given** la tabla messages existe  
**When** se ejecuta la migración  
**Then** existen:

```sql
CREATE INDEX IF NOT EXISTS idx_messages_unindexed 
ON messages(created_at) 
WHERE is_indexed = 0;

CREATE INDEX IF NOT EXISTS idx_messages_summary_ref 
ON messages(summary_ref) 
WHERE summary_ref IS NOT NULL;
```

#### Scenario: idx_messages_unindexed exists
**Given** migraciones ejecutadas  
**When** `SELECT name FROM sqlite_master WHERE type='index' AND name='idx_messages_unindexed'`  
**Then** devuelve el índice

#### Scenario: idx_messages_summary_ref exists
**Given** migraciones ejecutadas  
**When** `SELECT name FROM sqlite_master WHERE type='index' AND name='idx_messages_summary_ref'`  
**Then** devuelve el índice

### Requirement: Migration SHALL drop legacy tables

**Given** migraciones ejecutadas  
**When** se migra  
**Then** las tablas `memories`, `memory_embeddings`, `memories_fts` NO existen

#### Scenario: Legacy tables dropped
**Given** migraciones ejecutadas  
**When** `SELECT name FROM sqlite_master WHERE type='table' AND name IN ('memories','memory_embeddings','memories_fts')`  
**Then** devuelve 0 filas

### Requirement: MemoryRepo::create SHALL store episodic card

**Given** un MemoryRepo sobre una conexión  
**When** `create(pool, &content, tokens_count, &metadata)`  
**Then** inserta una fila en `memory` con UUID generado  
**And** devuelve el `Memory` creado con id, content, tokens_count, metadata, created_at

#### Scenario: Create memory card
**Given** pool con esquema migrado  
**When** `MemoryRepo::create(pool, "Ficha...", 200, r#"{"tags":["rust","sqlite"]}"#)`  
**Then** el resultado tiene `content` = "Ficha..."  
**And** `tokens_count` = 200  
**And** `metadata` contiene los tags

### Requirement: MemoryRepo::search_by_vector SHALL query vec_memory

**Given** memorias con embeddings en vec_memory  
**When** `search_by_vector(pool, &embedding, limit, budget_tokens)`  
**Then** devuelve las `Memory` más similares por cosine similarity  
**And** la suma de `tokens_count` de los resultados NO excede `budget_tokens`

#### Scenario: Search returns top-K within budget
**Given** 3 memorias con tokens_count [100, 200, 300]  
**When** `search_by_vector(pool, &embedding, 10, 250)`  
**Then** devuelve las 2 primeras (100+200 <= 250)  
**And** NO incluye la tercera (suma excedería 250)

#### Scenario: Empty results when no matches
**Given** vec_memory vacía  
**When** `search_by_vector(pool, &embedding, 10, 5000)`  
**Then** devuelve vec vacío

### Requirement: MemoryRepo::list SHALL paginate memory cards

**Given** memorias en la tabla  
**When** `list(pool, limit, offset)`  
**Then** devuelve `(Vec<Memory>, total_count)` ordenado por created_at DESC  
**And** limit entre 1 y 100, offset >= 0

#### Scenario: List paginates correctly
**Given** 5 memorias  
**When** `list(pool, 2, 0)`  
**Then** devuelve 2 items, total = 5

### Requirement: MemoryRepo::delete SHALL remove card and vector

**Given** una memoria existe en `memory` y `vec_memory`  
**When** `delete(pool, &id)`  
**Then** elimina la fila de `memory`  
**And** elimina el vector de `vec_memory` (rowid correspondiente)  
**And** devuelve true si existía, false si no

#### Scenario: Delete existing
**Given** memoria con id "mem-1" existe  
**When** `delete(pool, "mem-1")`  
**Then** devuelve true  
**And** `SELECT COUNT(*) FROM memory WHERE id = 'mem-1'` = 0  
**And** `SELECT COUNT(*) FROM vec_memory WHERE rowid = '...'` = 0 (rowid correspondiente)

#### Scenario: Delete non-existent
**Given** no memoria con id "nonexistent"  
**When** `delete(pool, "nonexistent")`  
**Then** devuelve false