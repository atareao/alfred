use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: String,
    pub profile_id: String,
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct ContactsRepo;

impl ContactsRepo {
    pub fn create(conn: &Connection, contact: &Contact) -> SqlResult<()> {
        conn.execute(
            "INSERT INTO contacts (id, profile_id, name, phone, email, notes, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                contact.id, contact.profile_id, contact.name,
                contact.phone, contact.email, contact.notes,
                contact.created_at, contact.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn find_by_id(conn: &Connection, id: &str) -> SqlResult<Option<Contact>> {
        let mut stmt = conn.prepare(
            "SELECT id, profile_id, name, phone, email, notes, created_at, updated_at
             FROM contacts WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(Contact {
                id: row.get(0)?,
                profile_id: row.get(1)?,
                name: row.get(2)?,
                phone: row.get(3)?,
                email: row.get(4)?,
                notes: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        match rows.next() {
            Some(Ok(contact)) => Ok(Some(contact)),
            _ => Ok(None),
        }
    }

    /// Search contacts by name, phone, or email using LIKE.
    pub fn search(conn: &Connection, profile_id: &str, query: &str) -> SqlResult<Vec<Contact>> {
        let pattern = format!("%{}%", query);
        let mut stmt = conn.prepare(
            "SELECT id, profile_id, name, phone, email, notes, created_at, updated_at
             FROM contacts
             WHERE profile_id = ?1 AND (name LIKE ?2 OR phone LIKE ?2 OR email LIKE ?2)
             ORDER BY name ASC",
        )?;
        let rows = stmt.query_map(params![profile_id, pattern], |row| {
            Ok(Contact {
                id: row.get(0)?,
                profile_id: row.get(1)?,
                name: row.get(2)?,
                phone: row.get(3)?,
                email: row.get(4)?,
                notes: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        let mut contacts = Vec::new();
        for row in rows {
            contacts.push(row?);
        }
        Ok(contacts)
    }

    pub fn update(
        conn: &Connection,
        id: &str,
        name: Option<&str>,
        phone: Option<&str>,
        email: Option<&str>,
        notes: Option<&str>,
    ) -> SqlResult<()> {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE contacts SET
                name = COALESCE(?1, name),
                phone = COALESCE(?2, phone),
                email = COALESCE(?3, email),
                notes = COALESCE(?4, notes),
                updated_at = ?5
             WHERE id = ?6",
            params![name, phone, email, notes, now, id],
        )?;
        Ok(())
    }

    pub fn delete(conn: &Connection, id: &str) -> SqlResult<()> {
        conn.execute("DELETE FROM contacts WHERE id = ?1", params![id])?;
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

    fn sample_contact(id: &str, name: &str, phone: Option<&str>, email: Option<&str>) -> Contact {
        Contact {
            id: id.into(),
            profile_id: "profile-1".into(),
            name: name.into(),
            phone: phone.map(|s| s.into()),
            email: email.map(|s| s.into()),
            notes: None,
            created_at: "2026-09-23T00:00:00Z".into(),
            updated_at: "2026-09-23T00:00:00Z".into(),
        }
    }

    #[test]
    fn test_create_contact() {
        let conn = setup();
        let contact = sample_contact(
            "c1",
            "Juan Pérez",
            Some("+34 600 000 000"),
            Some("juan@example.com"),
        );
        ContactsRepo::create(&conn, &contact).unwrap();
        let found = ContactsRepo::find_by_id(&conn, "c1").unwrap().unwrap();
        assert_eq!(found.name, "Juan Pérez");
        assert_eq!(found.email.unwrap(), "juan@example.com");
    }

    #[test]
    fn test_search_by_name() {
        let conn = setup();
        ContactsRepo::create(&conn, &sample_contact("c2", "María García", None, None)).unwrap();
        ContactsRepo::create(&conn, &sample_contact("c3", "Carlos López", None, None)).unwrap();
        ContactsRepo::create(&conn, &sample_contact("c4", "Ana Martínez", None, None)).unwrap();

        let results = ContactsRepo::search(&conn, "profile-1", "María").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "María García");

        let results = ContactsRepo::search(&conn, "profile-1", "ez").unwrap();
        assert_eq!(results.len(), 2); // López & Martínez both contain "ez"
    }

    #[test]
    fn test_search_by_phone() {
        let conn = setup();
        ContactsRepo::create(
            &conn,
            &sample_contact("c5", "Pedro", Some("+34 611 111 111"), None),
        )
        .unwrap();
        ContactsRepo::create(
            &conn,
            &sample_contact("c6", "Laura", Some("+34 622 222 222"), None),
        )
        .unwrap();

        let results = ContactsRepo::search(&conn, "profile-1", "611").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Pedro");
    }

    #[test]
    fn test_search_by_email() {
        let conn = setup();
        ContactsRepo::create(
            &conn,
            &sample_contact("c7", "Luis", None, Some("luis@work.com")),
        )
        .unwrap();
        ContactsRepo::create(
            &conn,
            &sample_contact("c8", "Elena", None, Some("elena@personal.com")),
        )
        .unwrap();

        let results = ContactsRepo::search(&conn, "profile-1", "work").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Luis");
    }

    #[test]
    fn test_update_contact() {
        let conn = setup();
        ContactsRepo::create(&conn, &sample_contact("c9", "Nombre Original", None, None)).unwrap();
        ContactsRepo::update(
            &conn,
            "c9",
            Some("Nombre Nuevo"),
            Some("+34 600 000 001"),
            None,
            Some("Nota importante"),
        )
        .unwrap();
        let found = ContactsRepo::find_by_id(&conn, "c9").unwrap().unwrap();
        assert_eq!(found.name, "Nombre Nuevo");
        assert_eq!(found.phone.unwrap(), "+34 600 000 001");
        assert_eq!(found.notes.unwrap(), "Nota importante");
    }

    #[test]
    fn test_search_no_results() {
        let conn = setup();
        ContactsRepo::create(&conn, &sample_contact("c10", "Solo yo", None, None)).unwrap();
        let results = ContactsRepo::search(&conn, "profile-1", "ZzzNadie").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_delete_contact() {
        let conn = setup();
        ContactsRepo::create(&conn, &sample_contact("c11", "Para borrar", None, None)).unwrap();
        ContactsRepo::delete(&conn, "c11").unwrap();
        let found = ContactsRepo::find_by_id(&conn, "c11").unwrap();
        assert!(found.is_none());
    }
}
