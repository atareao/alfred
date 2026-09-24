use rusqlite::{params, Connection};

/// Add FTS5 triggers to keep indexes up to date
pub fn create_fts_triggers(conn: &Connection) -> Result<(), rusqlite::Error> {
    // Messages FTS triggers
    conn.execute_batch(
        "CREATE TRIGGER IF NOT EXISTS messages_fts_ai AFTER INSERT ON messages BEGIN
            INSERT INTO messages_fts(rowid, content) VALUES (new.rowid, new.content);
        END;
        CREATE TRIGGER IF NOT EXISTS messages_fts_ad AFTER DELETE ON messages BEGIN
            INSERT INTO messages_fts(messages_fts, rowid, content) VALUES('delete', old.rowid, old.content);
        END;
        CREATE TRIGGER IF NOT EXISTS messages_fts_au AFTER UPDATE ON messages BEGIN
            INSERT INTO messages_fts(messages_fts, rowid, content) VALUES('delete', old.rowid, old.content);
            INSERT INTO messages_fts(rowid, content) VALUES (new.rowid, new.content);
        END;"
    )?;

    // Memories FTS triggers
    conn.execute_batch(
        "CREATE TRIGGER IF NOT EXISTS memories_fts_ai AFTER INSERT ON memories BEGIN
            INSERT INTO memories_fts(rowid, content) VALUES (new.rowid, new.content);
        END;
        CREATE TRIGGER IF NOT EXISTS memories_fts_ad AFTER DELETE ON memories BEGIN
            INSERT INTO memories_fts(memories_fts, rowid, content) VALUES('delete', old.rowid, old.content);
        END;
        CREATE TRIGGER IF NOT EXISTS memories_fts_au AFTER UPDATE ON memories BEGIN
            INSERT INTO memories_fts(memories_fts, rowid, content) VALUES('delete', old.rowid, old.content);
            INSERT INTO memories_fts(rowid, content) VALUES (new.rowid, new.content);
        END;"
    )?;

    // Notes FTS triggers
    conn.execute_batch(
        "CREATE TRIGGER IF NOT EXISTS notes_ai AFTER INSERT ON notes BEGIN
            INSERT INTO notes_fts(rowid, content) VALUES (new.rowid, new.content);
        END;
        CREATE TRIGGER IF NOT EXISTS notes_ad AFTER DELETE ON notes BEGIN
            INSERT INTO notes_fts(notes_fts, rowid, content) VALUES('delete', old.rowid, old.content);
        END;
        CREATE TRIGGER IF NOT EXISTS notes_au AFTER UPDATE ON notes BEGIN
            INSERT INTO notes_fts(notes_fts, rowid, content) VALUES('delete', old.rowid, old.content);
            INSERT INTO notes_fts(rowid, content) VALUES (new.rowid, new.content);
        END;"
    )?;

    tracing::info!("FTS5 triggers created");
    Ok(())
}

/// Search messages using FTS5
pub fn search_messages_fts(
    conn: &Connection,
    query: &str,
    limit: i64,
) -> Result<Vec<(String, String, f64)>, rusqlite::Error> {
    let actual_limit = limit.clamp(1, 100);
    // Sanitize FTS5 query: escape special chars and add prefix matching
    let sanitized: String = query
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect();
    if sanitized.trim().is_empty() {
        return Ok(Vec::new());
    }
    // Use simple prefix matching
    let fts_query = sanitized
        .split_whitespace()
        .map(|w| format!("{}*", w))
        .collect::<Vec<_>>()
        .join(" AND ");

    let mut stmt = conn.prepare(
        "SELECT m.id, m.content, rank
         FROM messages_fts
         JOIN messages m ON messages_fts.rowid = m.rowid
         WHERE messages_fts MATCH ?1
         ORDER BY rank
         LIMIT ?2",
    )?;
    let mut rows = stmt.query(params![fts_query, actual_limit])?;
    let mut results = Vec::new();
    while let Some(row) = rows.next()? {
        results.push((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, f64>(2)?,
        ));
    }
    Ok(results)
}

/// Search memories using FTS5
pub fn search_memories_fts(
    conn: &Connection,
    query: &str,
    limit: i64,
) -> Result<Vec<(String, String, f64)>, rusqlite::Error> {
    let actual_limit = limit.clamp(1, 100);
    let sanitized: String = query
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect();
    if sanitized.trim().is_empty() {
        return Ok(Vec::new());
    }
    let fts_query = sanitized
        .split_whitespace()
        .map(|w| format!("{}*", w))
        .collect::<Vec<_>>()
        .join(" AND ");

    let mut stmt = conn.prepare(
        "SELECT m.id, m.content, rank
         FROM memories_fts
         JOIN memories m ON memories_fts.rowid = m.rowid
         WHERE memories_fts MATCH ?1
         ORDER BY rank
         LIMIT ?2",
    )?;
    let mut rows = stmt.query(params![fts_query, actual_limit])?;
    let mut results = Vec::new();
    while let Some(row) = rows.next()? {
        results.push((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, f64>(2)?,
        ));
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL DEFAULT '',
                role TEXT NOT NULL DEFAULT 'user',
                content TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(content, content=messages, content_rowid=rowid);
            CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                profile_id TEXT NOT NULL DEFAULT '',
                content TEXT NOT NULL DEFAULT '',
                category TEXT NOT NULL DEFAULT 'general',
                source TEXT NOT NULL DEFAULT 'manual',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(content, content=memories, content_rowid=rowid);
            CREATE TABLE IF NOT EXISTS notes (
                id TEXT PRIMARY KEY,
                profile_id TEXT NOT NULL DEFAULT '',
                content TEXT NOT NULL DEFAULT '',
                category TEXT NOT NULL DEFAULT 'idea',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(content, content=notes, content_rowid=rowid);"
        ).unwrap();
        create_fts_triggers(&conn).unwrap();
        conn
    }

    #[test]
    fn test_trigger_inserts_into_fts() {
        let conn = setup();
        conn.execute(
            "INSERT INTO messages (id, conversation_id, role, content) VALUES (?1, ?2, ?3, ?4)",
            params!["m1", "c1", "user", "receta de pasta"],
        )
        .unwrap();

        let results = search_messages_fts(&conn, "receta", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "m1");
    }

    #[test]
    fn test_trigger_deletes_from_fts() {
        let conn = setup();
        conn.execute(
            "INSERT INTO messages (id, conversation_id, role, content) VALUES (?1, ?2, ?3, ?4)",
            params!["m1", "c1", "user", "test content"],
        )
        .unwrap();
        conn.execute("DELETE FROM messages WHERE id = 'm1'", [])
            .unwrap();

        let results = search_messages_fts(&conn, "test", 10).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_search_memories_fts() {
        let conn = setup();
        conn.execute(
            "INSERT INTO memories (id, profile_id, content, category, source) VALUES (?1, ?2, ?3, ?4, ?5)",
            params!["mem1", "p1", "A Alfred le gusta el café", "fact", "manual"],
        ).unwrap();

        let results = search_memories_fts(&conn, "café", 10).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_empty_query_returns_empty() {
        let conn = setup();
        let results = search_messages_fts(&conn, "", 10).unwrap();
        assert!(results.is_empty());
    }
}
