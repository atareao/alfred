use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub struct MemoryConsolidator {
    db: Arc<Mutex<Connection>>,
}

impl MemoryConsolidator {
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self { db }
    }

    /// Run nightly consolidation.
    /// Returns a summary of what was cleaned up.
    pub fn consolidate(&self) -> Result<String, String> {
        let conn = self.db.lock().map_err(|e| e.to_string())?;
        let mut report = vec![];

        // 1. Clean orphan message_embeddings
        let orphaned = conn
            .execute(
                "DELETE FROM message_embeddings WHERE id NOT IN (SELECT id FROM messages)",
                [],
            )
            .map_err(|e| e.to_string())?;
        if orphaned > 0 {
            report.push(format!(
                "🧹 Eliminados {} embeddings huérfanos de mensajes",
                orphaned
            ));
        }

        // 2. Clean orphan memory_embeddings
        let orphaned_mem = conn
            .execute(
                "DELETE FROM memory_embeddings WHERE id NOT IN (SELECT id FROM memories)",
                [],
            )
            .map_err(|e| e.to_string())?;
        if orphaned_mem > 0 {
            report.push(format!(
                "🧹 Eliminados {} embeddings huérfanos de memorias",
                orphaned_mem
            ));
        }

        // 3. Clean old conversations (more than 30 days with no messages)
        let old = conn
            .execute(
                "DELETE FROM conversations WHERE id IN (
                    SELECT c.id FROM conversations c
                    LEFT JOIN messages m ON m.conversation_id = c.id
                    GROUP BY c.id
                    HAVING MAX(m.created_at) < datetime('now', '-30 days')
                )",
                [],
            )
            .map_err(|e| e.to_string())?;
        if old > 0 {
            report.push(format!("🗑️ Archivadas {} conversaciones antiguas", old));
        }

        if report.is_empty() {
            report.push("✅ Todo en orden — no hay datos que limpiar".to_string());
        }

        Ok(report.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::run_migrations;

    fn setup() -> (Arc<Mutex<Connection>>, MemoryConsolidator) {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let db = Arc::new(Mutex::new(conn));
        let consolidator = MemoryConsolidator::new(db.clone());
        (db, consolidator)
    }

    #[test]
    fn test_consolidate_empty_db() {
        let (_, consolidator) = setup();
        let result = consolidator.consolidate().unwrap();
        assert!(result.contains("Todo en orden"));
    }

    #[test]
    fn test_consolidate_cleans_orphaned_message_embeddings() {
        let (db, consolidator) = setup();
        {
            let conn = db.lock().unwrap();
            // Insert orphan message_embedding (no corresponding message)
            conn.execute(
                "INSERT INTO message_embeddings (id, embedding) VALUES ('orphan-msg', '[]')",
                [],
            )
            .unwrap();
        }
        let result = consolidator.consolidate().unwrap();
        assert!(result.contains("embeddings huérfanos"));
        // Verify it was deleted
        let conn = db.lock().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM message_embeddings WHERE id = 'orphan-msg'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_consolidate_cleans_orphaned_memory_embeddings() {
        let (db, consolidator) = setup();
        {
            let conn = db.lock().unwrap();
            conn.execute(
                "INSERT INTO memory_embeddings (id, embedding) VALUES ('orphan-mem', '[]')",
                [],
            )
            .unwrap();
        }
        let result = consolidator.consolidate().unwrap();
        assert!(result.contains("embeddings huérfanos"));
        let conn = db.lock().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memory_embeddings WHERE id = 'orphan-mem'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }
}
