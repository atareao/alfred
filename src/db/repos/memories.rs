use chrono::Utc;
use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::models::Memory;

pub struct MemoriesRepo;

impl MemoriesRepo {
    pub fn create(
        conn: &Connection,
        profile_id: &str,
        content: &str,
        category: &str,
        source: &str,
    ) -> Result<Memory, rusqlite::Error> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO memories (id, profile_id, content, category, source, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, profile_id, content, category, source, now],
        )?;
        Ok(Memory {
            id,
            profile_id: profile_id.to_string(),
            content: content.to_string(),
            category: category.to_string(),
            source: source.to_string(),
            embedding_id: None,
            created_at: now,
        })
    }

    pub fn find_by_id(conn: &Connection, id: &str) -> Result<Option<Memory>, rusqlite::Error> {
        let mut stmt = conn.prepare(
            "SELECT id, profile_id, content, category, source, embedding_id, created_at \
             FROM memories WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => Ok(Some(Memory {
                id: row.get(0)?,
                profile_id: row.get(1)?,
                content: row.get(2)?,
                category: row.get(3)?,
                source: row.get(4)?,
                embedding_id: row.get(5)?,
                created_at: row.get(6)?,
            })),
            None => Ok(None),
        }
    }

    pub fn list(
        conn: &Connection,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<Memory>, i64), rusqlite::Error> {
        let actual_limit = limit.clamp(1, 100);
        let actual_offset = offset.max(0);

        let total: i64 = conn.query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))?;

        let mut stmt = conn.prepare(
            "SELECT id, profile_id, content, category, source, embedding_id, created_at \
             FROM memories ORDER BY created_at DESC LIMIT ?1 OFFSET ?2",
        )?;
        let mut rows = stmt.query(params![actual_limit, actual_offset])?;

        let mut items = Vec::new();
        while let Some(row) = rows.next()? {
            items.push(Memory {
                id: row.get(0)?,
                profile_id: row.get(1)?,
                content: row.get(2)?,
                category: row.get(3)?,
                source: row.get(4)?,
                embedding_id: row.get(5)?,
                created_at: row.get(6)?,
            });
        }
        Ok((items, total))
    }

    pub fn delete(conn: &Connection, id: &str) -> Result<bool, rusqlite::Error> {
        let deleted = conn.execute("DELETE FROM memories WHERE id = ?1", params![id])?;
        Ok(deleted > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::repos::profiles::ProfilesRepo;
    use crate::db::schema::run_migrations;

    fn setup_with_profile() -> (Connection, String) {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let profile = ProfilesRepo::get_or_create(&conn).unwrap();
        (conn, profile.id)
    }

    #[test]
    fn test_create_memory() {
        let (conn, profile_id) = setup_with_profile();
        let mem =
            MemoriesRepo::create(&conn, &profile_id, "Remember this", "fact", "manual").unwrap();
        assert!(!mem.id.is_empty());
        assert_eq!(mem.content, "Remember this");
        assert_eq!(mem.category, "fact");
    }

    #[test]
    fn test_create_memory_default_category() {
        let (conn, profile_id) = setup_with_profile();
        let mem =
            MemoriesRepo::create(&conn, &profile_id, "Default cat", "general", "chat").unwrap();
        assert_eq!(mem.category, "general");
        assert_eq!(mem.source, "chat");
    }

    #[test]
    fn test_find_by_id_found() {
        let (conn, profile_id) = setup_with_profile();
        let created =
            MemoriesRepo::create(&conn, &profile_id, "Find me", "fact", "manual").unwrap();
        let found = MemoriesRepo::find_by_id(&conn, &created.id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().content, "Find me");
    }

    #[test]
    fn test_find_by_id_not_found() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let found = MemoriesRepo::find_by_id(&conn, "nonexistent").unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_list_memories() {
        let (conn, profile_id) = setup_with_profile();
        MemoriesRepo::create(&conn, &profile_id, "First", "general", "manual").unwrap();
        MemoriesRepo::create(&conn, &profile_id, "Second", "fact", "manual").unwrap();
        let (data, total) = MemoriesRepo::list(&conn, 10, 0).unwrap();
        assert_eq!(data.len(), 2);
        assert_eq!(total, 2);
    }

    #[test]
    fn test_list_memories_with_pagination() {
        let (conn, profile_id) = setup_with_profile();
        for i in 0..5 {
            MemoriesRepo::create(
                &conn,
                &profile_id,
                &format!("Memory {}", i),
                "general",
                "manual",
            )
            .unwrap();
        }
        let (page1, total) = MemoriesRepo::list(&conn, 2, 0).unwrap();
        assert_eq!(page1.len(), 2);
        assert_eq!(total, 5);

        let (page2, _) = MemoriesRepo::list(&conn, 2, 2).unwrap();
        assert_eq!(page2.len(), 2);

        let (page3, _) = MemoriesRepo::list(&conn, 2, 4).unwrap();
        assert_eq!(page3.len(), 1);
    }

    #[test]
    fn test_delete_found() {
        let (conn, profile_id) = setup_with_profile();
        let mem =
            MemoriesRepo::create(&conn, &profile_id, "To delete", "general", "manual").unwrap();
        assert!(MemoriesRepo::delete(&conn, &mem.id).unwrap());
    }

    #[test]
    fn test_delete_not_found() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        assert!(!MemoriesRepo::delete(&conn, "nonexistent").unwrap());
    }
}
