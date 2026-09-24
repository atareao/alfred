use crate::db::repos::events::EventsRepo;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

/// Worker that analyzes upcoming events with a location and generates travel
/// preparation suggestions (weather, restaurants, itinerary) N days before
/// the trip.
#[allow(dead_code)]
pub struct TravelPrepWorker {
    db: Arc<Mutex<Connection>>,
    days_before: u32, // default: 3
}

impl TravelPrepWorker {
    pub fn new(db: Arc<Mutex<Connection>>, days_before: u32) -> Self {
        Self { db, days_before }
    }

    /// Find upcoming trips (events with location) within the next 30 days.
    pub fn find_upcoming_trips(&self, profile_id: &str) -> Result<Vec<String>, String> {
        let conn = self.db.lock().map_err(|e| e.to_string())?;
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

        // Look ahead 30 days for events with location
        let end = (chrono::Utc::now() + chrono::Duration::days(30))
            .format("%Y-%m-%d")
            .to_string();
        let events = EventsRepo::list_by_date_range(&conn, profile_id, &today, &end)
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

    fn setup_db() -> Arc<Mutex<Connection>> {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn.execute(
            "INSERT INTO profiles (id, name, preferences) VALUES ('profile-1', 'Test User', '{}')",
            [],
        )
        .unwrap();
        Arc::new(Mutex::new(conn))
    }

    #[test]
    fn test_no_trips() {
        let db = setup_db();
        let worker = TravelPrepWorker::new(db, 3);
        let trips = worker.find_upcoming_trips("profile-1").unwrap();
        assert!(
            trips.is_empty(),
            "Expected no trips when there are no events with location"
        );
    }

    #[test]
    fn test_find_trips() {
        let db = setup_db();
        let now = chrono::Utc::now();
        let today = now.format("%Y-%m-%d").to_string();

        // Insert an event WITH location (should be picked up)
        {
            let conn = db.lock().unwrap();
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
            EventsRepo::create(&conn, &event).unwrap();

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
            EventsRepo::create(&conn, &event_no_loc).unwrap();
        }

        let worker = TravelPrepWorker::new(db, 3);
        let trips = worker.find_upcoming_trips("profile-1").unwrap();

        assert_eq!(trips.len(), 1, "Should find exactly 1 trip (with location)");
        assert!(
            trips[0].contains("Viaje a Paris"),
            "Trip description should contain the event title"
        );
        assert!(
            trips[0].contains("Paris"),
            "Trip description should contain the location"
        );
    }

    #[test]
    fn test_prepare_for_trip_returns_markdown() {
        let db = setup_db();
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
    }
}
