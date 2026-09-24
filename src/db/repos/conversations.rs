use chrono::Utc;
use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::models::Conversation;

pub struct ConversationsRepo;

impl ConversationsRepo {
    pub fn create(conn: &Connection, title: &str) -> Result<Conversation, rusqlite::Error> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO conversations (id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, title, now, now],
        )?;
        Ok(Conversation {
            id,
            title: title.to_string(),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn find_by_id(
        conn: &Connection,
        id: &str,
    ) -> Result<Option<Conversation>, rusqlite::Error> {
        let mut stmt = conn
            .prepare("SELECT id, title, created_at, updated_at FROM conversations WHERE id = ?1")?;
        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => Ok(Some(Conversation {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
            })),
            None => Ok(None),
        }
    }

    pub fn list(
        conn: &Connection,
        limit: i64,
        cursor: Option<&str>,
    ) -> Result<(Vec<Conversation>, Option<String>), rusqlite::Error> {
        let actual_limit = limit.clamp(1, 100);
        let query = match cursor {
            Some(_) => {
                "SELECT id, title, created_at, updated_at FROM conversations \
                 WHERE created_at < ?1 ORDER BY created_at DESC LIMIT ?2"
            }
            None => {
                "SELECT id, title, created_at, updated_at FROM conversations \
                 ORDER BY created_at DESC LIMIT ?1"
            }
        };
        let mut stmt = conn.prepare(query)?;

        let rows: Vec<Conversation> = if let Some(c) = cursor {
            let mut rows = stmt.query(params![c, actual_limit + 1])?;
            let mut items = Vec::new();
            while let Some(row) = rows.next()? {
                items.push(Conversation {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                });
            }
            items
        } else {
            let mut rows = stmt.query(params![actual_limit + 1])?;
            let mut items = Vec::new();
            while let Some(row) = rows.next()? {
                items.push(Conversation {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                });
            }
            items
        };

        let has_more = rows.len() > actual_limit as usize;
        let data: Vec<Conversation> = if has_more {
            rows[..actual_limit as usize].to_vec()
        } else {
            rows
        };
        let next_cursor = if has_more {
            data.last().map(|c| c.created_at.clone())
        } else {
            None
        };

        Ok((data, next_cursor))
    }

    pub fn find_first(conn: &Connection) -> Result<Option<Conversation>, rusqlite::Error> {
        let mut stmt = conn.prepare(
            "SELECT id, title, created_at, updated_at FROM conversations ORDER BY created_at ASC LIMIT 1",
        )?;
        let mut rows = stmt.query([])?;
        match rows.next()? {
            Some(row) => Ok(Some(Conversation {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
            })),
            None => Ok(None),
        }
    }

    pub fn update(
        conn: &Connection,
        id: &str,
        title: Option<&str>,
    ) -> Result<Option<Conversation>, rusqlite::Error> {
        let now = Utc::now().to_rfc3339();
        let updated = conn.execute(
            "UPDATE conversations SET title = COALESCE(?1, title), updated_at = ?2 WHERE id = ?3",
            params![title, now, id],
        )?;
        if updated == 0 {
            return Ok(None);
        }
        Self::find_by_id(conn, id)
    }

    pub fn delete(conn: &Connection, id: &str) -> Result<bool, rusqlite::Error> {
        // Also delete associated messages
        conn.execute(
            "DELETE FROM messages WHERE conversation_id = ?1",
            params![id],
        )?;
        let deleted = conn.execute("DELETE FROM conversations WHERE id = ?1", params![id])?;
        Ok(deleted > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::run_migrations;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn test_create_conversation() {
        let conn = setup();
        let conv = ConversationsRepo::create(&conn, "Test Title").unwrap();
        assert!(!conv.id.is_empty());
        assert_eq!(conv.title, "Test Title");
    }

    #[test]
    fn test_find_by_id_found() {
        let conn = setup();
        let created = ConversationsRepo::create(&conn, "Find Me").unwrap();
        let found = ConversationsRepo::find_by_id(&conn, &created.id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().title, "Find Me");
    }

    #[test]
    fn test_find_by_id_not_found() {
        let conn = setup();
        let found = ConversationsRepo::find_by_id(&conn, "nonexistent").unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_list_with_pagination() {
        let conn = setup();
        ConversationsRepo::create(&conn, "A").unwrap();
        ConversationsRepo::create(&conn, "B").unwrap();
        ConversationsRepo::create(&conn, "C").unwrap();

        let (data, cursor) = ConversationsRepo::list(&conn, 2, None).unwrap();
        assert_eq!(data.len(), 2);
        assert!(cursor.is_some());

        // Fetch next page
        let (data2, cursor2) = ConversationsRepo::list(&conn, 2, cursor.as_deref()).unwrap();
        assert_eq!(data2.len(), 1);
        assert!(cursor2.is_none());
    }

    #[test]
    fn test_update_conversation_title() {
        let conn = setup();
        let conv = ConversationsRepo::create(&conn, "Old Title").unwrap();
        let updated = ConversationsRepo::update(&conn, &conv.id, Some("New Title"))
            .unwrap()
            .unwrap();
        assert_eq!(updated.title, "New Title");
    }

    #[test]
    fn test_delete_found() {
        let conn = setup();
        let created = ConversationsRepo::create(&conn, "To Delete").unwrap();
        let deleted = ConversationsRepo::delete(&conn, &created.id).unwrap();
        assert!(deleted);
        let found = ConversationsRepo::find_by_id(&conn, &created.id).unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_delete_not_found() {
        let conn = setup();
        let deleted = ConversationsRepo::delete(&conn, "nonexistent").unwrap();
        assert!(!deleted);
    }
}
