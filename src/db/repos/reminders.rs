use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reminder {
    pub id: String,
    pub profile_id: String,
    pub text: String,
    pub datetime: String,
    pub status: String,
    pub created_at: String,
}

pub struct RemindersRepo;

impl RemindersRepo {
    pub fn create(conn: &Connection, reminder: &Reminder) -> SqlResult<()> {
        conn.execute(
            "INSERT INTO reminders (id, profile_id, text, datetime, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                reminder.id,
                reminder.profile_id,
                reminder.text,
                reminder.datetime,
                reminder.status,
                reminder.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn find_by_id(conn: &Connection, id: &str) -> SqlResult<Option<Reminder>> {
        let mut stmt = conn.prepare(
            "SELECT id, profile_id, text, datetime, status, created_at
             FROM reminders WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(Reminder {
                id: row.get(0)?,
                profile_id: row.get(1)?,
                text: row.get(2)?,
                datetime: row.get(3)?,
                status: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        match rows.next() {
            Some(Ok(reminder)) => Ok(Some(reminder)),
            _ => Ok(None),
        }
    }

    pub fn list(
        conn: &Connection,
        profile_id: &str,
        status: Option<&str>,
    ) -> SqlResult<Vec<Reminder>> {
        let mut sql = String::from(
            "SELECT id, profile_id, text, datetime, status, created_at
             FROM reminders WHERE profile_id = ?1",
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> =
            vec![Box::new(profile_id.to_string())];

        if let Some(s) = status {
            param_values.push(Box::new(s.to_string()));
            sql.push_str(&format!(" AND status = ?{}", param_values.len()));
        }

        sql.push_str(" ORDER BY datetime ASC");

        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            Ok(Reminder {
                id: row.get(0)?,
                profile_id: row.get(1)?,
                text: row.get(2)?,
                datetime: row.get(3)?,
                status: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        let mut reminders = Vec::new();
        for row in rows {
            reminders.push(row?);
        }
        Ok(reminders)
    }

    pub fn dismiss(conn: &Connection, id: &str) -> SqlResult<()> {
        conn.execute(
            "UPDATE reminders SET status = 'dismissed' WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    /// Snooze a reminder to a new date/time and reset status to pending.
    pub fn snooze(conn: &Connection, id: &str, new_datetime: &str) -> SqlResult<()> {
        conn.execute(
            "UPDATE reminders SET status = 'pending', datetime = ?1 WHERE id = ?2",
            params![new_datetime, id],
        )?;
        Ok(())
    }

    pub fn delete(conn: &Connection, id: &str) -> SqlResult<()> {
        conn.execute("DELETE FROM reminders WHERE id = ?1", params![id])?;
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

    fn sample_reminder(id: &str, text: &str, datetime: &str) -> Reminder {
        Reminder {
            id: id.into(),
            profile_id: "profile-1".into(),
            text: text.into(),
            datetime: datetime.into(),
            status: "pending".into(),
            created_at: "2026-09-23T00:00:00Z".into(),
        }
    }

    #[test]
    fn test_create_reminder() {
        let conn = setup();
        let reminder = sample_reminder("rem-1", "Comprar leche", "2026-09-24T10:00:00Z");
        RemindersRepo::create(&conn, &reminder).unwrap();
        let found = RemindersRepo::find_by_id(&conn, "rem-1").unwrap().unwrap();
        assert_eq!(found.text, "Comprar leche");
        assert_eq!(found.status, "pending");
    }

    #[test]
    fn test_dismiss_reminder() {
        let conn = setup();
        RemindersRepo::create(
            &conn,
            &sample_reminder("rem-2", "Reunión", "2026-09-24T15:00:00Z"),
        )
        .unwrap();
        RemindersRepo::dismiss(&conn, "rem-2").unwrap();
        let found = RemindersRepo::find_by_id(&conn, "rem-2").unwrap().unwrap();
        assert_eq!(found.status, "dismissed");
    }

    #[test]
    fn test_snooze_reminder() {
        let conn = setup();
        RemindersRepo::create(
            &conn,
            &sample_reminder("rem-3", "Llamar", "2026-09-24T10:00:00Z"),
        )
        .unwrap();
        RemindersRepo::snooze(&conn, "rem-3", "2026-09-24T12:00:00Z").unwrap();
        let found = RemindersRepo::find_by_id(&conn, "rem-3").unwrap().unwrap();
        assert_eq!(found.status, "pending");
        assert_eq!(found.datetime, "2026-09-24T12:00:00Z");
    }

    #[test]
    fn test_list_reminders_by_status() {
        let conn = setup();
        RemindersRepo::create(&conn, &sample_reminder("r1", "A", "2026-09-24T10:00:00Z")).unwrap();
        RemindersRepo::create(&conn, &sample_reminder("r2", "B", "2026-09-24T11:00:00Z")).unwrap();
        RemindersRepo::dismiss(&conn, "r2").unwrap();

        let pending = RemindersRepo::list(&conn, "profile-1", Some("pending")).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, "r1");

        let dismissed = RemindersRepo::list(&conn, "profile-1", Some("dismissed")).unwrap();
        assert_eq!(dismissed.len(), 1);
        assert_eq!(dismissed[0].id, "r2");
    }

    #[test]
    fn test_delete_reminder() {
        let conn = setup();
        RemindersRepo::create(&conn, &sample_reminder("r3", "C", "2026-09-24T12:00:00Z")).unwrap();
        RemindersRepo::delete(&conn, "r3").unwrap();
        let found = RemindersRepo::find_by_id(&conn, "r3").unwrap();
        assert!(found.is_none());
    }
}
