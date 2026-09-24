use chrono::Utc;
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::models::Profile;

pub struct ProfilesRepo;

impl ProfilesRepo {
    pub fn get_or_create(conn: &Connection) -> Result<Profile, rusqlite::Error> {
        let mut stmt = conn.prepare(
            "SELECT id, name, avatar_url, preferences, created_at, updated_at FROM profiles LIMIT 1",
        )?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            let prefs: String = row.get(3)?;
            return Ok(Profile {
                id: row.get(0)?,
                name: row.get(1)?,
                avatar_url: row.get(2)?,
                preferences: serde_json::from_str(&prefs).unwrap_or(json!({})),
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            });
        }

        // Create default profile
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let default_prefs = json!({}).to_string();
        conn.execute(
            "INSERT INTO profiles (id, name, avatar_url, preferences, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id,
                "Alfred User",
                Option::<String>::None,
                default_prefs,
                now,
                now
            ],
        )?;

        Ok(Profile {
            id,
            name: "Alfred User".to_string(),
            avatar_url: None,
            preferences: json!({}),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn update(
        conn: &Connection,
        name: Option<&str>,
        avatar_url: Option<&str>,
        preferences: Option<&Value>,
    ) -> Result<Profile, rusqlite::Error> {
        // Ensure a profile exists first, so we can update by ID
        let existing = Self::get_or_create(conn)?;
        let now = Utc::now().to_rfc3339();

        let new_name = name.unwrap_or(&existing.name);
        let new_avatar = avatar_url.or(existing.avatar_url.as_deref());
        let new_prefs = preferences
            .map(|v| v.to_string())
            .unwrap_or_else(|| existing.preferences.to_string());

        conn.execute(
            "UPDATE profiles SET name = ?1, avatar_url = ?2, preferences = ?3, updated_at = ?4 WHERE id = ?5",
            params![new_name, new_avatar, new_prefs, now, existing.id],
        )?;

        Ok(Profile {
            id: existing.id,
            name: new_name.to_string(),
            avatar_url: new_avatar.map(|s| s.to_string()),
            preferences: preferences.cloned().unwrap_or(existing.preferences),
            created_at: existing.created_at,
            updated_at: now,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::run_migrations;
    use serde_json::json;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn test_get_or_create_creates_default() {
        let conn = setup();
        let profile = ProfilesRepo::get_or_create(&conn).unwrap();
        assert_eq!(profile.name, "Alfred User");
        assert!(profile.avatar_url.is_none());
    }

    #[test]
    fn test_get_or_create_is_idempotent() {
        let conn = setup();
        let p1 = ProfilesRepo::get_or_create(&conn).unwrap();
        let p2 = ProfilesRepo::get_or_create(&conn).unwrap();
        assert_eq!(p1.id, p2.id);
    }

    #[test]
    fn test_update_profile() {
        let conn = setup();
        let profile = ProfilesRepo::update(
            &conn,
            Some("New Name"),
            None,
            Some(&json!({"theme": "dark"})),
        )
        .unwrap();
        assert_eq!(profile.name, "New Name");
        assert_eq!(profile.preferences["theme"], "dark");
    }

    #[test]
    fn test_update_profile_partial_no_name() {
        let conn = setup();
        let profile =
            ProfilesRepo::update(&conn, None, None, Some(&json!({"lang": "es"}))).unwrap();
        assert_eq!(profile.name, "Alfred User");
        assert_eq!(profile.preferences["lang"], "es");
    }
}
