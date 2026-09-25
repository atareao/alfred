use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

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
    pub async fn create(pool: &SqlitePool, event: &Event) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO events (id, profile_id, title, description, start_time, end_time, location, scope, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )
        .bind(&event.id)
        .bind(&event.profile_id)
        .bind(&event.title)
        .bind(&event.description)
        .bind(&event.start_time)
        .bind(&event.end_time)
        .bind(&event.location)
        .bind(&event.scope)
        .bind(&event.created_at)
        .bind(&event.updated_at)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Event>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, profile_id, title, description, start_time, end_time, location, scope, created_at, updated_at
             FROM events WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|r| Event {
            id: r.get(0),
            profile_id: r.get(1),
            title: r.get(2),
            description: r.get(3),
            start_time: r.get(4),
            end_time: r.get(5),
            location: r.get(6),
            scope: r.get(7),
            created_at: r.get(8),
            updated_at: r.get(9),
        }))
    }

    pub async fn list_by_date_range(
        pool: &SqlitePool,
        profile_id: &str,
        start: &str,
        end: &str,
    ) -> Result<Vec<Event>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, profile_id, title, description, start_time, end_time, location, scope, created_at, updated_at
             FROM events WHERE profile_id = ?1 AND start_time >= ?2 AND end_time <= ?3
             ORDER BY start_time ASC",
        )
        .bind(profile_id)
        .bind(start)
        .bind(end)
        .fetch_all(pool)
        .await?;

        let events: Vec<Event> = rows
            .iter()
            .map(|r| Event {
                id: r.get(0),
                profile_id: r.get(1),
                title: r.get(2),
                description: r.get(3),
                start_time: r.get(4),
                end_time: r.get(5),
                location: r.get(6),
                scope: r.get(7),
                created_at: r.get(8),
                updated_at: r.get(9),
            })
            .collect();

        Ok(events)
    }

    pub async fn update(
        pool: &SqlitePool,
        id: &str,
        title: Option<&str>,
        description: Option<&str>,
        location: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE events SET title = COALESCE(?1, title), description = COALESCE(?2, description), location = COALESCE(?3, location), updated_at = ?4 WHERE id = ?5",
        )
        .bind(title)
        .bind(description)
        .bind(location)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn find_free_slots(
        pool: &SqlitePool,
        profile_id: &str,
        date: &str,
        duration_min: i64,
    ) -> Result<Vec<(String, String)>, sqlx::Error> {
        let day_start = format!("{}T00:00:00Z", date);
        let day_end = format!("{}T23:59:59Z", date);
        let events = Self::list_by_date_range(pool, profile_id, &day_start, &day_end).await?;

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
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    async fn setup() -> Result<SqlitePool, sqlx::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await?;
        sqlx::migrate::Migrator::new(std::path::Path::new("migrations"))
            .await
            .unwrap()
            .run(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO profiles (id, name, preferences) VALUES ('profile-1', 'Test', '{}')",
        )
        .execute(&pool)
        .await?;
        Ok(pool)
    }

    #[tokio::test]
    async fn test_create_event() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
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
        EventsRepo::create(&pool, &event).await.unwrap();
        let found = EventsRepo::find_by_id(&pool, "evt-1").await?.unwrap();
        assert_eq!(found.title, "Reunión");
        assert_eq!(found.scope, "shared");

        Ok(())
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        let found = EventsRepo::find_by_id(&pool, "nonexistent").await.unwrap();
        assert!(found.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_list_by_date_range() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
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
        EventsRepo::create(&pool, &event).await.unwrap();
        let events = EventsRepo::list_by_date_range(
            &pool,
            "profile-1",
            "2026-09-24T00:00:00Z",
            "2026-09-24T23:59:59Z",
        )
        .await?;
        assert_eq!(events.len(), 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_list_by_date_range_outside() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
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
        EventsRepo::create(&pool, &event).await.unwrap();
        let events = EventsRepo::list_by_date_range(
            &pool,
            "profile-1",
            "2026-09-24T00:00:00Z",
            "2026-09-24T23:59:59Z",
        )
        .await?;
        assert!(events.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_update_event() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
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
        EventsRepo::create(&pool, &event).await.unwrap();
        EventsRepo::update(&pool, "evt-4", Some("Actualizado"), None, Some("Oficina")).await?;
        let found = EventsRepo::find_by_id(&pool, "evt-4").await?.unwrap();
        assert_eq!(found.title, "Actualizado");
        assert_eq!(found.location.unwrap(), "Oficina");

        Ok(())
    }

    #[tokio::test]
    async fn test_find_free_slots() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
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
        EventsRepo::create(&pool, &event).await.unwrap();
        let slots = EventsRepo::find_free_slots(&pool, "profile-1", "2026-09-24", 30).await?;
        assert!(!slots.is_empty());
        // Should have a slot before 10:00 and after 11:00
        assert!(slots.len() >= 2);

        Ok(())
    }
}
