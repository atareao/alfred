use crate::db::repos::events::EventsRepo;
use sqlx::SqlitePool;

/// Worker that analyzes upcoming events with a location and generates travel
/// preparation suggestions (weather, restaurants, itinerary) N days before
/// the trip.
#[allow(dead_code)]
pub struct TravelPrepWorker {
    db: SqlitePool,
    days_before: u32, // default: 3
}

impl TravelPrepWorker {
    pub fn new(db: SqlitePool, days_before: u32) -> Self {
        Self { db, days_before }
    }

    /// Find upcoming trips (events with location) within the next 30 days.
    pub async fn find_upcoming_trips(&self, profile_id: &str) -> Result<Vec<String>, String> {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

        // Look ahead 30 days for events with location
        let end = (chrono::Utc::now() + chrono::Duration::days(30))
            .format("%Y-%m-%d")
            .to_string();
        let events = EventsRepo::list_by_date_range(&self.db, profile_id, &today, &end)
            .await
            .map_err(|e| e.to_string())?;

        let trips: Vec<String> = events
            .into_iter()
            .filter(|e| e.location.is_some())
            .map(|e| {
                format!(
                    "{} en {}",
                    e.title,
                    e.location.as_deref().unwrap_or_default()
                )
            })
            .collect();

        Ok(trips)
    }

    /// Generate travel preparation for a specific event.
    /// Returns markdown with weather, suggestions, etc.
    pub fn prepare_for_trip(&self, event_title: &str, location: &str) -> Result<String, String> {
        let mut parts = vec![];
        parts.push(format!("## 🧳 Preparación de viaje: {}", event_title));
        parts.push(format!("📍 Destino: {}", location));
        parts.push("\n### 🌤️ Clima".to_string());
        parts.push("_(requiere OPENWEATHER_API_KEY para datos reales)_".to_string());
        parts.push("\n### 🏨 Sugerencias".to_string());
        parts.push("- Revisa el clima antes de empacar".to_string());
        parts.push("- Consulta restaurantes cercanos con `search_places`".to_string());
        parts.push("- Verifica el transporte disponible".to_string());
        Ok(parts.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::repos::events::Event;
    use crate::db::schema::run_migrations;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use sqlx::SqlitePool;

    async fn setup_db() -> Result<SqlitePool, sqlx::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await?;
        run_migrations(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO profiles (id, name, preferences) VALUES ('profile-1', 'Test User', '{}')",
        )
        .execute(&pool)
        .await?;
        Ok(pool)
    }

    #[tokio::test]
    async fn test_no_trips() -> Result<(), Box<dyn std::error::Error>> {
        let db = setup_db().await?;
        let worker = TravelPrepWorker::new(db, 3);
        let trips = worker.find_upcoming_trips("profile-1").await?;
        assert!(
            trips.is_empty(),
            "Expected no trips when there are no events with location"
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_find_trips() -> Result<(), Box<dyn std::error::Error>> {
        let db = setup_db().await?;
        let now = chrono::Utc::now();
        let today = now.format("%Y-%m-%d").to_string();

        // Insert an event WITH location (should be picked up)
        let event = Event {
            id: "evt-trip-1".into(),
            profile_id: "profile-1".into(),
            title: "Viaje a Paris".into(),
            description: None,
            start_time: format!("{}T10:00:00Z", today),
            end_time: format!("{}T11:00:00Z", today),
            location: Some("Paris, Francia".into()),
            scope: "shared".into(),
            created_at: now.to_rfc3339(),
            updated_at: now.to_rfc3339(),
        };
        EventsRepo::create(&db, &event).await?;

        // Insert an event WITHOUT location (should be ignored)
        let event_no_loc = Event {
            id: "evt-no-trip-1".into(),
            profile_id: "profile-1".into(),
            title: "Reunion local".into(),
            description: None,
            start_time: format!("{}T15:00:00Z", today),
            end_time: format!("{}T16:00:00Z", today),
            location: None,
            scope: "shared".into(),
            created_at: now.to_rfc3339(),
            updated_at: now.to_rfc3339(),
        };
        EventsRepo::create(&db, &event_no_loc).await?;

        let worker = TravelPrepWorker::new(db, 3);
        let trips = worker.find_upcoming_trips("profile-1").await?;

        assert_eq!(trips.len(), 1, "Should find exactly 1 trip (with location)");
        assert!(
            trips[0].contains("Viaje a Paris"),
            "Trip description should contain the event title"
        );
        assert!(
            trips[0].contains("Paris"),
            "Trip description should contain the location"
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_prepare_for_trip_returns_markdown() -> Result<(), Box<dyn std::error::Error>> {
        let db = setup_db().await?;
        let worker = TravelPrepWorker::new(db, 3);

        let result = worker.prepare_for_trip("Viaje a Paris", "Paris, Francia");
        assert!(result.is_ok());

        let markdown = result.unwrap();
        // Must be valid markdown with the expected sections
        assert!(
            markdown.contains("Preparación de viaje: Viaje a Paris"),
            "Should contain the event title"
        );
        assert!(
            markdown.contains("📍 Destino: Paris, Francia"),
            "Should contain the destination"
        );
        assert!(
            markdown.contains("🌤️ Clima"),
            "Should contain a weather section"
        );
        assert!(
            markdown.contains("🏨 Sugerencias"),
            "Should contain a suggestions section"
        );
        assert!(
            markdown.contains("OPENWEATHER_API_KEY"),
            "Should mention the API key requirement"
        );
        Ok(())
    }
}
