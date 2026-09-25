use crate::db::repos::events::EventsRepo;
use crate::db::repos::reminders::RemindersRepo;
use crate::db::repos::tasks::TasksRepo;
use sqlx::SqlitePool;

/// Worker that generates a daily briefing with events, tasks, weather, and reminders.
#[allow(dead_code)]
pub struct BriefingWorker {
    db: SqlitePool,
    weather_api_key: Option<String>,
}

impl BriefingWorker {
    pub fn new(db: SqlitePool, weather_api_key: Option<String>) -> Self {
        Self {
            db,
            weather_api_key,
        }
    }

    /// Generate today's briefing. Returns a markdown-formatted string.
    pub async fn generate(&self) -> Result<String, String> {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let tomorrow = (chrono::Utc::now() + chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();

        // Use full RFC3339 for correct lexicographic comparison with stored datetimes
        let day_start = format!("{}T00:00:00Z", today);
        let day_end = format!("{}T00:00:00Z", tomorrow);

        let mut parts: Vec<String> = Vec::new();

        // ── Events today ──────────────────────────────────────────────────
        let events = EventsRepo::list_by_date_range(&self.db, "profile-id", &day_start, &day_end)
            .await
            .map_err(|e| e.to_string())?;
        if !events.is_empty() {
            parts.push("## 📅 Eventos de hoy".to_string());
            for e in &events {
                parts.push(format!(
                    "- **{}** — {} ({})",
                    e.title, e.start_time, e.scope
                ));
            }
        } else {
            parts.push("📅 No tienes eventos hoy.".to_string());
        }

        // ── Tasks pending ─────────────────────────────────────────────────
        let tasks = TasksRepo::list(&self.db, "profile-id", Some("pending"), None, None, None)
            .await
            .map_err(|e| e.to_string())?;
        if !tasks.is_empty() {
            parts.push(String::new());
            parts.push("## ✅ Tareas pendientes".to_string());
            for t in &tasks {
                parts.push(format!("- [ ] {} (prioridad: {})", t.content, t.priority));
            }
        }

        // ── Reminders for today ───────────────────────────────────────────
        let reminders = RemindersRepo::list(&self.db, "profile-id", Some("pending"))
            .await
            .map_err(|e| e.to_string())?;
        // Filter in-memory: keep only reminders whose datetime starts with today
        let reminders_today: Vec<_> = reminders
            .into_iter()
            .filter(|r| r.datetime.starts_with(&today))
            .collect();
        if !reminders_today.is_empty() {
            parts.push(String::new());
            parts.push("## ⏰ Recordatorios".to_string());
            for r in &reminders_today {
                parts.push(format!("- **{}** — {}", r.text, r.datetime));
            }
        }

        Ok(parts.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::repos::events::Event;
    use crate::db::repos::reminders::Reminder;
    use crate::db::repos::tasks::Task;
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
        // Insert a profile with the placeholder id used by the worker
        sqlx::query(
            "INSERT INTO profiles (id, name, preferences) VALUES ('profile-id', 'Test User', '{}')",
        )
        .execute(&pool)
        .await?;
        Ok(pool)
    }

    #[tokio::test]
    async fn test_briefing_generates_no_panic() -> Result<(), Box<dyn std::error::Error>> {
        let db = setup_db().await?;
        let worker = BriefingWorker::new(db, None);
        let result = worker.generate().await;
        assert!(result.is_ok());
        let briefing = result.unwrap();
        // Should be a non-empty string
        assert!(!briefing.is_empty());
        // Should mention no events (we inserted none for today)
        assert!(briefing.contains("No tienes eventos hoy"));
        Ok(())
    }

    #[tokio::test]
    async fn test_briefing_with_data() -> Result<(), Box<dyn std::error::Error>> {
        let db = setup_db().await?;
        let now = chrono::Utc::now();
        let today = now.format("%Y-%m-%d").to_string();

        // Insert an event for today
        let event = Event {
            id: "evt-brief-1".into(),
            profile_id: "profile-id".into(),
            title: "Reunión de equipo".into(),
            description: None,
            start_time: format!("{}T10:00:00Z", today),
            end_time: format!("{}T11:00:00Z", today),
            location: None,
            scope: "shared".into(),
            created_at: now.to_rfc3339(),
            updated_at: now.to_rfc3339(),
        };
        EventsRepo::create(&db, &event).await?;

        // Insert a pending task
        let task = Task {
            id: "task-brief-1".into(),
            profile_id: "profile-id".into(),
            content: "Comprar víveres".into(),
            status: "pending".into(),
            priority: "high".into(),
            project: None,
            due_date: None,
            scope: "shared".into(),
            created_at: now.to_rfc3339(),
            updated_at: now.to_rfc3339(),
        };
        TasksRepo::create(&db, &task).await?;

        // Insert a reminder for today
        let reminder = Reminder {
            id: "rem-brief-1".into(),
            profile_id: "profile-id".into(),
            text: "Llamar al dentista".into(),
            datetime: format!("{}T09:00:00Z", today),
            status: "pending".into(),
            created_at: now.to_rfc3339(),
        };
        RemindersRepo::create(&db, &reminder).await?;

        let worker = BriefingWorker::new(db, None);
        let result = worker.generate().await;
        assert!(result.is_ok());
        let briefing = result.unwrap();

        // Should contain the event
        assert!(
            briefing.contains("Reunión de equipo"),
            "Briefing should contain event title"
        );
        // Should contain the task
        assert!(
            briefing.contains("Comprar víveres"),
            "Briefing should contain task content"
        );
        assert!(
            briefing.contains("prioridad: high"),
            "Briefing should contain task priority"
        );
        // Should contain the reminder
        assert!(
            briefing.contains("Llamar al dentista"),
            "Briefing should contain reminder text"
        );
        Ok(())
    }
}
