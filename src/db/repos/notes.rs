use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub profile_id: String,
    pub content: String,
    pub category: String,
    pub tags: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct NotesRepo;

impl NotesRepo {
    pub fn create(conn: &Connection, note: &Note) -> SqlResult<()> {
        conn.execute(
            "INSERT INTO notes (id, profile_id, content, category, tags, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                note.id,
                note.profile_id,
                note.content,
                note.category,
                note.tags,
                note.created_at,
                note.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn find_by_id(conn: &Connection, id: &str) -> SqlResult<Option<Note>> {
        let mut stmt = conn.prepare(
            "SELECT id, profile_id, content, category, tags, created_at, updated_at
             FROM notes WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(Note {
                id: row.get(0)?,
                profile_id: row.get(1)?,
                content: row.get(2)?,
                category: row.get(3)?,
                tags: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;
        match rows.next() {
            Some(Ok(note)) => Ok(Some(note)),
            _ => Ok(None),
        }
    }

    pub fn list(
        conn: &Connection,
        profile_id: &str,
        category: Option<&str>,
    ) -> SqlResult<Vec<Note>> {
        let mut sql = String::from(
            "SELECT id, profile_id, content, category, tags, created_at, updated_at
             FROM notes WHERE profile_id = ?1",
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> =
            vec![Box::new(profile_id.to_string())];

        if let Some(cat) = category {
            param_values.push(Box::new(cat.to_string()));
            sql.push_str(&format!(" AND category = ?{}", param_values.len()));
        }

        sql.push_str(" ORDER BY created_at DESC");

        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            Ok(Note {
                id: row.get(0)?,
                profile_id: row.get(1)?,
                content: row.get(2)?,
                category: row.get(3)?,
                tags: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;
        let mut notes = Vec::new();
        for row in rows {
            notes.push(row?);
        }
        Ok(notes)
    }

    pub fn update(
        conn: &Connection,
        id: &str,
        content: Option<&str>,
        category: Option<&str>,
        tags: Option<&str>,
    ) -> SqlResult<()> {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE notes SET
                content = COALESCE(?1, content),
                category = COALESCE(?2, category),
                tags = COALESCE(?3, tags),
                updated_at = ?4
             WHERE id = ?5",
            params![content, category, tags, now, id],
        )?;
        Ok(())
    }

    pub fn delete(conn: &Connection, id: &str) -> SqlResult<()> {
        conn.execute("DELETE FROM notes WHERE id = ?1", params![id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::run_migrations;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn.execute(
            "INSERT INTO profiles (id, name, preferences) VALUES ('profile-1', 'Test', '{}')",
            [],
        )
        .unwrap();
        conn
    }

    fn sample_note(id: &str, content: &str, category: &str, tags: Option<&str>) -> Note {
        Note {
            id: id.into(),
            profile_id: "profile-1".into(),
            content: content.into(),
            category: category.into(),
            tags: tags.map(|s| s.into()),
            created_at: "2026-09-23T00:00:00Z".into(),
            updated_at: "2026-09-23T00:00:00Z".into(),
        }
    }

    #[test]
    fn test_create_note() {
        let conn = setup();
        let note = sample_note(
            "note-1",
            "Recordar comprar pan",
            "idea",
            Some("compras,casa"),
        );
        NotesRepo::create(&conn, &note).unwrap();
        let found = NotesRepo::find_by_id(&conn, "note-1").unwrap().unwrap();
        assert_eq!(found.content, "Recordar comprar pan");
        assert_eq!(found.category, "idea");
        assert_eq!(found.tags.unwrap(), "compras,casa");
    }

    #[test]
    fn test_list_by_category() {
        let conn = setup();
        NotesRepo::create(&conn, &sample_note("n1", "Idea genial", "idea", None)).unwrap();
        NotesRepo::create(&conn, &sample_note("n2", "Diario del día", "journal", None)).unwrap();
        NotesRepo::create(&conn, &sample_note("n3", "Dato curioso", "fact", None)).unwrap();
        NotesRepo::create(&conn, &sample_note("n4", "Otra idea", "idea", None)).unwrap();

        let ideas = NotesRepo::list(&conn, "profile-1", Some("idea")).unwrap();
        assert_eq!(ideas.len(), 2);

        let journals = NotesRepo::list(&conn, "profile-1", Some("journal")).unwrap();
        assert_eq!(journals.len(), 1);
    }

    #[test]
    fn test_list_all() {
        let conn = setup();
        NotesRepo::create(&conn, &sample_note("n5", "Nota 1", "idea", None)).unwrap();
        NotesRepo::create(&conn, &sample_note("n6", "Nota 2", "fact", None)).unwrap();

        let all = NotesRepo::list(&conn, "profile-1", None).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_update_note() {
        let conn = setup();
        NotesRepo::create(&conn, &sample_note("n7", "Original", "idea", None)).unwrap();
        NotesRepo::update(
            &conn,
            "n7",
            Some("Actualizado"),
            Some("journal"),
            Some("importante"),
        )
        .unwrap();
        let found = NotesRepo::find_by_id(&conn, "n7").unwrap().unwrap();
        assert_eq!(found.content, "Actualizado");
        assert_eq!(found.category, "journal");
        assert_eq!(found.tags.unwrap(), "importante");
    }

    #[test]
    fn test_delete_note() {
        let conn = setup();
        NotesRepo::create(&conn, &sample_note("n8", "Para borrar", "todo", None)).unwrap();
        NotesRepo::delete(&conn, "n8").unwrap();
        let found = NotesRepo::find_by_id(&conn, "n8").unwrap();
        assert!(found.is_none());
    }
}
