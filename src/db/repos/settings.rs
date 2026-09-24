use rusqlite::Connection;
use std::collections::HashMap;

pub struct SettingsRepo;

impl SettingsRepo {
    /// Get a single setting by key. Returns None if not found.
    pub fn get(conn: &Connection, key: &str) -> Result<Option<String>, rusqlite::Error> {
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query(rusqlite::params![key])?;
        match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }

    /// Set a setting (insert or update). Sets updated_at to current timestamp.
    pub fn set(conn: &Connection, key: &str, value: &str) -> Result<(), rusqlite::Error> {
        conn.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')",
            rusqlite::params![key, value],
        )?;
        Ok(())
    }

    /// Get all settings as a HashMap.
    pub fn get_all(conn: &Connection) -> Result<HashMap<String, String>, rusqlite::Error> {
        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut map = HashMap::new();
        for row in rows {
            let (k, v) = row?;
            map.insert(k, v);
        }
        Ok(map)
    }

    /// Delete a setting by key.
    pub fn delete(conn: &Connection, key: &str) -> Result<(), rusqlite::Error> {
        conn.execute(
            "DELETE FROM settings WHERE key = ?1",
            rusqlite::params![key],
        )?;
        Ok(())
    }

    /// Seed default settings values.
    pub fn seed_defaults(conn: &Connection) -> Result<(), rusqlite::Error> {
        conn.execute(
            "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
            rusqlite::params!["max_window_tokens", "10000"],
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
            rusqlite::params!["system_prompt", ""],
        )?;
        // STUB: collapse_prompt seeded as empty for RED phase.
        // TODO: Set proper Spanish default prompt in GREEN phase.
        conn.execute(
            "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
            rusqlite::params!["collapse_prompt", "Resume el siguiente texto manteniendo la información clave, los datos importantes y el contexto necesario. Sé conciso."],
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
            rusqlite::params!["message_page_size", "50"],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_default_value() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::schema::run_migrations(&conn).unwrap();
        let value = SettingsRepo::get(&conn, "max_window_tokens").unwrap();
        assert_eq!(value, Some("10000".to_string()));
    }

    #[test]
    fn test_get_nonexistent() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::schema::run_migrations(&conn).unwrap();
        let value = SettingsRepo::get(&conn, "nonexistent").unwrap();
        assert_eq!(value, None);
    }

    #[test]
    fn test_set_inserts_new() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::schema::run_migrations(&conn).unwrap();
        SettingsRepo::set(&conn, "foo", "bar").unwrap();
        let value = SettingsRepo::get(&conn, "foo").unwrap();
        assert_eq!(value, Some("bar".to_string()));
    }

    #[test]
    fn test_set_updates_existing() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::schema::run_migrations(&conn).unwrap();
        SettingsRepo::set(&conn, "foo", "bar").unwrap();
        SettingsRepo::set(&conn, "foo", "baz").unwrap();
        let value = SettingsRepo::get(&conn, "foo").unwrap();
        assert_eq!(value, Some("baz".to_string()));
    }

    #[test]
    fn test_get_all() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::schema::run_migrations(&conn).unwrap();
        let all = SettingsRepo::get_all(&conn).unwrap();
        assert!(all.contains_key("max_window_tokens"));
        assert!(all.contains_key("system_prompt"));
        assert!(all.contains_key("collapse_prompt"));
    }

    #[test]
    fn test_collapse_prompt_seeded() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::schema::run_migrations(&conn).unwrap();
        let value = SettingsRepo::get(&conn, "collapse_prompt").unwrap();
        assert!(
            value.is_some(),
            "collapse_prompt should be seeded after migrations"
        );
        let prompt = value.unwrap();
        assert!(
            !prompt.is_empty(),
            "collapse_prompt should have a non-empty default value"
        );
        assert!(
            prompt.contains("Resume"),
            "Default collapse_prompt should be in Spanish, containing 'Resume'"
        );
    }

    #[test]
    fn test_delete() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::schema::run_migrations(&conn).unwrap();
        SettingsRepo::set(&conn, "foo", "bar").unwrap();
        SettingsRepo::delete(&conn, "foo").unwrap();
        let value = SettingsRepo::get(&conn, "foo").unwrap();
        assert_eq!(value, None);
    }

    /// After seed_defaults(), the message_page_size setting must exist with value "50".
    /// RED: seed_defaults() does NOT yet seed message_page_size → this test WILL fail.
    #[test]
    fn test_message_page_size_seeded() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::schema::run_migrations(&conn).unwrap();
        let value = SettingsRepo::get(&conn, "message_page_size").unwrap();
        assert_eq!(value, Some("50".to_string()));
    }
}
