use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub profile_id: String,
    pub title: String,
    pub description: Option<String>,
    pub start_time: String,
    pub end_time: String,
    pub location: Option<String>,
    pub scope: String,
    pub created_at: String,
    pub updated_at: String,
}

pub struct EventsRepo;

impl EventsRepo {
    pub fn create(conn: &Connection, event: &Event) -> SqlResult<()> {
        conn.execute(
            "INSERT INTO events (id, profile_id, title, description, start_time, end_time, location, scope, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![event.id, event.profile_id, event.title, event.description, event.start_time, event.end_time, event.location, event.scope, event.created_at, event.updated_at],
        )?;
        Ok(())
    }

    pub fn find_by_id(conn: &Connection, id: &str) -> SqlResult<Option<Event>> {
        let mut stmt = conn.prepare(
            "SELECT id, profile_id, title, description, start_time, end_time, location, scope, created_at, updated_at
             FROM events WHERE id = ?1"
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(Event {
                id: row.get(0)?,
                profile_id: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                start_time: row.get(4)?,
                end_time: row.get(5)?,
                location: row.get(6)?,
                scope: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;
        match rows.next() {
            Some(Ok(event)) => Ok(Some(event)),
            _ => Ok(None),
        }
    }

    pub fn list_by_date_range(
        conn: &Connection,
        profile_id: &str,
        start: &str,
        end: &str,
    ) -> SqlResult<Vec<Event>> {
        let mut stmt = conn.prepare(
            "SELECT id, profile_id, title, description, start_time, end_time, location, scope, created_at, updated_at
             FROM events WHERE profile_id = ?1 AND start_time >= ?2 AND end_time <= ?3
             ORDER BY start_time ASC"
        )?;
        let rows = stmt.query_map(params![profile_id, start, end], |row| {
            Ok(Event {
                id: row.get(0)?,
                profile_id: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                start_time: row.get(4)?,
                end_time: row.get(5)?,
                location: row.get(6)?,
                scope: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;
        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }
        Ok(events)
    }

    pub fn update(
        conn: &Connection,
        id: &str,
        title: Option<&str>,
        description: Option<&str>,
        location: Option<&str>,
    ) -> SqlResult<()> {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE events SET title = COALESCE(?1, title), description = COALESCE(?2, description), location = COALESCE(?3, location), updated_at = ?4 WHERE id = ?5",
            params![title, description, location, now, id],
        )?;
        Ok(())
    }

    pub fn find_free_slots(
        conn: &Connection,
        profile_id: &str,
        date: &str,
        duration_min: i64,
    ) -> SqlResult<Vec<(String, String)>> {
        let day_start = format!("{}T00:00:00Z", date);
        let day_end = format!("{}T23:59:59Z", date);
        let events = Self::list_by_date_range(conn, profile_id, &day_start, &day_end)?;

        // Helper to parse a datetime string; expects RFC 3339 format (with trailing Z).
        fn parse_dt(s: &str) -> Option<chrono::DateTime<chrono::FixedOffset>> {
            chrono::DateTime::parse_from_rfc3339(s).ok()
        }

        let mut slots = Vec::new();
        let mut cursor = day_start.clone();

        for event in &events {
            if cursor < event.start_time {
                let gap_start = parse_dt(&cursor);
                let gap_end = parse_dt(&event.start_time);
                if let (Some(start), Some(end)) = (gap_start, gap_end) {
                    let gap_minutes = (end - start).num_minutes();
                    if gap_minutes >= duration_min {
                        slots.push((cursor.clone(), event.start_time.clone()));
                    }
                }
            }
            if cursor < event.end_time {
                cursor = event.end_time.clone();
            }
        }

        if let (Some(start), Some(end)) = (parse_dt(&cursor), parse_dt(&day_end)) {
            if (end - start).num_minutes() >= duration_min {
                slots.push((cursor, day_end));
            }
        }

        Ok(slots)
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

    #[test]
    fn test_create_event() {
        let conn = setup();
        let event = Event {
            id: "evt-1".into(),
            profile_id: "profile-1".into(),
            title: "Reunión".into(),
            description: None,
            start_time: "2026-09-24T10:00:00Z".into(),
            end_time: "2026-09-24T11:00:00Z".into(),
            location: None,
            scope: "shared".into(),
            created_at: "2026-09-23T00:00:00Z".into(),
            updated_at: "2026-09-23T00:00:00Z".into(),
        };
        EventsRepo::create(&conn, &event).unwrap();
        let found = EventsRepo::find_by_id(&conn, "evt-1").unwrap().unwrap();
        assert_eq!(found.title, "Reunión");
        assert_eq!(found.scope, "shared");
    }

    #[test]
    fn test_find_by_id_not_found() {
        let conn = setup();
        let found = EventsRepo::find_by_id(&conn, "nonexistent").unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_list_by_date_range() {
        let conn = setup();
        let event = Event {
            id: "evt-2".into(),
            profile_id: "profile-1".into(),
            title: "Cita".into(),
            description: None,
            start_time: "2026-09-24T15:00:00Z".into(),
            end_time: "2026-09-24T16:00:00Z".into(),
            location: None,
            scope: "personal".into(),
            created_at: "2026-09-23T00:00:00Z".into(),
            updated_at: "2026-09-23T00:00:00Z".into(),
        };
        EventsRepo::create(&conn, &event).unwrap();
        let events = EventsRepo::list_by_date_range(
            &conn,
            "profile-1",
            "2026-09-24T00:00:00Z",
            "2026-09-24T23:59:59Z",
        )
        .unwrap();
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_list_by_date_range_outside() {
        let conn = setup();
        let event = Event {
            id: "evt-3".into(),
            profile_id: "profile-1".into(),
            title: "Fuera de rango".into(),
            description: None,
            start_time: "2026-09-25T10:00:00Z".into(),
            end_time: "2026-09-25T11:00:00Z".into(),
            location: None,
            scope: "shared".into(),
            created_at: "2026-09-23T00:00:00Z".into(),
            updated_at: "2026-09-23T00:00:00Z".into(),
        };
        EventsRepo::create(&conn, &event).unwrap();
        let events = EventsRepo::list_by_date_range(
            &conn,
            "profile-1",
            "2026-09-24T00:00:00Z",
            "2026-09-24T23:59:59Z",
        )
        .unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn test_update_event() {
        let conn = setup();
        let event = Event {
            id: "evt-4".into(),
            profile_id: "profile-1".into(),
            title: "Original".into(),
            description: None,
            start_time: "2026-09-24T10:00:00Z".into(),
            end_time: "2026-09-24T11:00:00Z".into(),
            location: None,
            scope: "shared".into(),
            created_at: "2026-09-23T00:00:00Z".into(),
            updated_at: "2026-09-23T00:00:00Z".into(),
        };
        EventsRepo::create(&conn, &event).unwrap();
        EventsRepo::update(&conn, "evt-4", Some("Actualizado"), None, Some("Oficina")).unwrap();
        let found = EventsRepo::find_by_id(&conn, "evt-4").unwrap().unwrap();
        assert_eq!(found.title, "Actualizado");
        assert_eq!(found.location.unwrap(), "Oficina");
    }

    #[test]
    fn test_find_free_slots() {
        let conn = setup();
        let event = Event {
            id: "evt-5".into(),
            profile_id: "profile-1".into(),
            title: "Ocupado".into(),
            description: None,
            start_time: "2026-09-24T10:00:00Z".into(),
            end_time: "2026-09-24T11:00:00Z".into(),
            location: None,
            scope: "shared".into(),
            created_at: "".into(),
            updated_at: "".into(),
        };
        EventsRepo::create(&conn, &event).unwrap();
        let slots = EventsRepo::find_free_slots(&conn, "profile-1", "2026-09-24", 30).unwrap();
        assert!(!slots.is_empty());
        // Should have a slot before 10:00 and after 11:00
        assert!(slots.len() >= 2);
    }
}
