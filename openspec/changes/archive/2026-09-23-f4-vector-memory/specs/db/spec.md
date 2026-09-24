# DB Spec — f4-vector-memory

## ADDED: sqlite-vec integration

Registrar la extensión sqlite-vec en la conexión SQLite al iniciar. Si no está disponible, continuar sin ella (fallback a solo FTS5).

```rust
// src/db/vector.rs
pub fn register_vector_ext(conn: &Connection) -> Result<(), rusqlite::Error> {
    // Cargar extensión sqlite_vec si está disponible
    // Envolver en try/catch para graceful degradation
    unsafe { conn.load_extension_enable()?; }
    let result = conn.execute_batch("SELECT load_extension('vec0');");
    unsafe { conn.load_extension_disable()?; }
    match result {
        Ok(_) => tracing::info!("sqlite-vec registered successfully"),
        Err(e) => tracing::warn!("sqlite-vec not available ({}), falling back to FTS5 only", e),
    }
    Ok(())
}
```

## ADDED: FTS5 triggers

Añadir triggers para mantener los índices FTS5 actualizados automáticamente:

```sql
-- Triggers para messages_fts
CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages BEGIN
  INSERT INTO messages_fts(rowid, content) VALUES (new.rowid, new.content);
END;

CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages BEGIN
  INSERT INTO messages_fts(messages_fts, rowid, content) VALUES('delete', old.rowid, old.content);
END;

CREATE TRIGGER IF NOT EXISTS messages_au AFTER UPDATE ON messages BEGIN
  INSERT INTO messages_fts(messages_fts, rowid, content) VALUES('delete', old.rowid, old.content);
  INSERT INTO messages_fts(rowid, content) VALUES (new.rowid, new.content);
END;

-- Triggers para memories_fts
CREATE TRIGGER IF NOT EXISTS memories_ai AFTER INSERT ON memories BEGIN
  INSERT INTO memories_fts(rowid, content) VALUES (new.rowid, new.content);
END;

CREATE TRIGGER IF NOT EXISTS memories_ad AFTER DELETE ON memories BEGIN
  INSERT INTO memories_fts(memories_fts, rowid, content) VALUES('delete', old.rowid, old.content);
END;
```

## ADDED: Vector storage functions

```rust
// src/db/vector.rs
pub fn store_message_embedding(conn: &Connection, message_id: &str, embedding: &[f32]) -> Result<()>;
pub fn store_memory_embedding(conn: &Connection, memory_id: &str, embedding: &[f32]) -> Result<()>;
pub fn search_message_vectors(conn: &Connection, embedding: &[f32], limit: i64) -> Result<Vec<(String, f32)>>;
pub fn search_memory_vectors(conn: &Connection, embedding: &[f32], limit: i64) -> Result<Vec<(String, f32)>>;
```

## Scenarios (BDD)

### Scenario: sqlite-vec registers without error
- **Given** a SQLite connection
- **When** register_vector_ext() is called
- **Then** returns Ok even if vec is not available (graceful)

### Scenario: FTS5 triggers fire on insert
- **Given** a message is inserted
- **When** the AFTER INSERT trigger fires
- **Then** the message appears in messages_fts

### Scenario: Vector store and search
- **Given** an embedding is stored
- **When** searching with a similar vector
- **Then** returns the stored item with a similarity score