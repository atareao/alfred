use crate::db::repos::events::EventsRepo;
use crate::db::repos::reminders::RemindersRepo;
use crate::db::repos::tasks::TasksRepo;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

/// Worker that generates a daily briefing with events, tasks, weather, and reminders.
#[allow(dead_code)]
pub struct BriefingWorker {
    db: Arc<Mutex<Connection>>,
    weather_api_key: Option<String>,
}

impl BriefingWorker {
    pub fn new(db: Arc<Mutex<Connection>>, weather_api_key: Option<String>) -> Self {
        Self {
            db,
            weather_api_key,
        }
    }

    /// Generate today's briefing. Returns a markdown-formatted string.
    pub fn generate(&self) -> Result<String, String> {
        let conn = self.db.lock().map_err(|e| e.to_string())?;
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let tomorrow = (chrono::Utc::now() + chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();

        // Use full RFC3339 for correct lexicographic comparison with stored datetimes
        let day_start = format!("{}T00:00:00Z", today);
        let day_end = format!("{}T00:00:00Z", tomorrow);

        let mut parts: Vec<String> = Vec::new();

        // ── Events today ──────────────────────────────────────────────────
        let events = EventsRepo::list_by_date_range(&conn, "profile-id", &day_start, &day_end)
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
        let tasks = TasksRepo::list(&conn, "profile-id", Some("pending"), None, None, None)
            .map_err(|e| e.to_string())?;
        if !tasks.is_empty() {
            parts.push(String::new());
            parts.push("## ✅ Tareas pendientes".to_string());
            for t in &tasks {
                parts.push(format!("- [ ] {} (prioridad: {})", t.content, t.priority));
            }
        }

        // ── Reminders for today ───────────────────────────────────────────
        let reminders =
            RemindersRepo::list(&conn, "profile-id", Some("pending")).map_err(|e| e.to_string())?;
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

    fn setup_db() -> Arc<Mutex<Connection>> {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        // Insert a profile with the placeholder id used by the worker
        conn.execute(
            "INSERT INTO profiles (id, name, preferences) VALUES ('profile-id', 'Test User', '{}')",
            [],
        )
        .unwrap();
        Arc::new(Mutex::new(conn))
    }

    #[test]
    fn test_briefing_generates_no_panic() {
        let db = setup_db();
        let worker = BriefingWorker::new(db, None);
        let result = worker.generate();
        assert!(result.is_ok());
        let briefing = result.unwrap();
        // Should be a non-empty string
        assert!(!briefing.is_empty());
        // Should mention no events (we inserted none for today)
        assert!(briefing.contains("No tienes eventos hoy"));
    }

    #[test]
    fn test_briefing_with_data() {
        let db = setup_db();
        let now = chrono::Utc::now();
        let today = now.format("%Y-%m-%d").to_string();

        // Insert an event for today
        {
            let conn = db.lock().unwrap();
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
            EventsRepo::create(&conn, &event).unwrap();

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
            TasksRepo::create(&conn, &task).unwrap();

            // Insert a reminder for today
            let reminder = Reminder {
                id: "rem-brief-1".into(),
                profile_id: "profile-id".into(),
                text: "Llamar al dentista".into(),
                datetime: format!("{}T09:00:00Z", today),
                status: "pending".into(),
                created_at: now.to_rfc3339(),
            };
            RemindersRepo::create(&conn, &reminder).unwrap();
        }

        let worker = BriefingWorker::new(db, None);
        let result = worker.generate();
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
    }
}
