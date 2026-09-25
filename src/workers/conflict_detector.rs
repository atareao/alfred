use crate::db::repos::events::{Event, EventsRepo};
use chrono::NaiveDateTime;
use sqlx::SqlitePool;

#[derive(Debug, Clone, PartialEq)]
pub enum ConflictSeverity {
    Warning,  // gap < 30 min but no overlap
    Critical, // actual overlap
}

#[derive(Debug, Clone)]
pub struct ConflictAlert {
    pub event_a: Event,
    pub event_b: Event,
    pub gap_minutes: i64,
    pub severity: ConflictSeverity,
}

pub struct ConflictDetector {
    db: SqlitePool,
    min_gap_minutes: i64, // default: 30
}

impl ConflictDetector {
    pub fn new(db: SqlitePool) -> Self {
        Self {
            db,
            min_gap_minutes: 30,
        }
    }

    /// Check for conflicts on a specific date.
    /// Returns a list of ConflictAlerts.
    pub async fn check_date(
        &self,
        profile_id: &str,
        date: &str,
    ) -> Result<Vec<ConflictAlert>, String> {
        let next_day = format_tomorrow(date);

        let events = EventsRepo::list_by_date_range(&self.db, profile_id, date, &next_day)
            .await
            .map_err(|e| e.to_string())?;

        let mut alerts = vec![];
        // Sort by start_time
        let mut sorted = events.clone();
        sorted.sort_by(|a, b| a.start_time.cmp(&b.start_time));

        for i in 0..sorted.len().saturating_sub(1) {
            let current = &sorted[i];
            let next = &sorted[i + 1];

            let current_end = NaiveDateTime::parse_from_str(&current.end_time, "%Y-%m-%dT%H:%M:%S")
                .map_err(|e| e.to_string())?;
            let next_start = NaiveDateTime::parse_from_str(&next.start_time, "%Y-%m-%dT%H:%M:%S")
                .map_err(|e| e.to_string())?;

            // Check for overlap
            if next_start < current_end {
                let overlap = (current_end - next_start).num_minutes();
                alerts.push(ConflictAlert {
                    event_a: current.clone(),
                    event_b: next.clone(),
                    gap_minutes: -overlap, // negative = overlap
                    severity: ConflictSeverity::Critical,
                });
            } else {
                let gap = (next_start - current_end).num_minutes();
                if gap < self.min_gap_minutes {
                    alerts.push(ConflictAlert {
                        event_a: current.clone(),
                        event_b: next.clone(),
                        gap_minutes: gap,
                        severity: ConflictSeverity::Warning,
                    });
                }
            }
        }

        Ok(alerts)
    }
}

fn format_tomorrow(date: &str) -> String {
    let d = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .unwrap_or_else(|_| chrono::Utc::now().date_naive());
    (d + chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
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
            "INSERT INTO profiles (id, name, preferences, created_at, updated_at)
             VALUES ('profile-1', 'Test', '{}', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        )
        .execute(&pool)
        .await?;
        Ok(pool)
    }

    fn make_event(id: &str, start: &str, end: &str) -> Event {
        Event {
            id: id.to_string(),
            profile_id: "profile-1".to_string(),
            title: format!("Event {}", id),
            description: None,
            start_time: start.to_string(),
            end_time: end.to_string(),
            location: None,
            scope: "personal".to_string(),
            created_at: "2026-09-24T00:00:00Z".to_string(),
            updated_at: "2026-09-24T00:00:00Z".to_string(),
        }
    }

    #[tokio::test]
    async fn test_no_conflicts_empty() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup_db().await?;
        let detector = ConflictDetector::new(pool);
        let alerts = detector.check_date("profile-1", "2026-09-24").await?;
        assert!(alerts.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_no_conflicts_well_spaced() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup_db().await?;
        // Two events with 1h gap — no alert expected
        let e1 = make_event("evt-1", "2026-09-24T09:00:00", "2026-09-24T10:00:00");
        let e2 = make_event("evt-2", "2026-09-24T11:00:00", "2026-09-24T12:00:00");
        EventsRepo::create(&pool, &e1).await?;
        EventsRepo::create(&pool, &e2).await?;
        let detector = ConflictDetector::new(pool);
        let alerts = detector.check_date("profile-1", "2026-09-24").await?;
        assert!(alerts.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_warning_tight_gap() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup_db().await?;
        // Two events with 10min gap (< 30) — should return Warning
        let e1 = make_event("evt-1", "2026-09-24T09:00:00", "2026-09-24T10:00:00");
        let e2 = make_event("evt-2", "2026-09-24T10:10:00", "2026-09-24T11:00:00");
        EventsRepo::create(&pool, &e1).await?;
        EventsRepo::create(&pool, &e2).await?;
        let detector = ConflictDetector::new(pool);
        let alerts = detector.check_date("profile-1", "2026-09-24").await?;
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].severity, ConflictSeverity::Warning);
        assert_eq!(alerts[0].gap_minutes, 10);
        Ok(())
    }

    #[tokio::test]
    async fn test_critical_overlap() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup_db().await?;
        // Two overlapping events — should return Critical
        let e1 = make_event("evt-1", "2026-09-24T09:00:00", "2026-09-24T10:30:00");
        let e2 = make_event("evt-2", "2026-09-24T10:00:00", "2026-09-24T11:00:00");
        EventsRepo::create(&pool, &e1).await?;
        EventsRepo::create(&pool, &e2).await?;
        let detector = ConflictDetector::new(pool);
        let alerts = detector.check_date("profile-1", "2026-09-24").await?;
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].severity, ConflictSeverity::Critical);
        assert!(alerts[0].gap_minutes < 0); // negative = overlap
        Ok(())
    }

    #[test]
    fn test_format_tomorrow() {
        let result = format_tomorrow("2026-09-24");
        assert_eq!(result, "2026-09-25");
    }
}
