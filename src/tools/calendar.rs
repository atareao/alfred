use async_trait::async_trait;
use chrono::Utc;
use rusqlite::Connection;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::db::repos::events::{Event, EventsRepo};
use crate::tools::permission::Permission;
use crate::tools::r#trait::{Tool, ToolError, ToolResult};

pub struct CalendarTool {
    db: Arc<Mutex<Connection>>,
}

impl CalendarTool {
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self { db }
    }

    async fn get_events(&self, args: Value) -> Result<ToolResult, ToolError> {
        let profile_id = args
            .get("profile_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing profile_id".into()))?;
        let start = args
            .get("start")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing start".into()))?;
        let end = args
            .get("end")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing end".into()))?;

        let conn = self
            .db
            .lock()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;
        let events = EventsRepo::list_by_date_range(&conn, profile_id, start, end)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        Ok(ToolResult {
            success: true,
            data: serde_json::to_value(events).unwrap_or_default(),
            message: None,
        })
    }

    async fn check_availability(&self, args: Value) -> Result<ToolResult, ToolError> {
        let profile_id = args
            .get("profile_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing profile_id".into()))?;
        let date = args
            .get("date")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing date".into()))?;
        let duration = args
            .get("duration")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| ToolError::InvalidArguments("Missing duration".into()))?;

        let conn = self
            .db
            .lock()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;
        let slots = EventsRepo::find_free_slots(&conn, profile_id, date, duration)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        let slots_json: Vec<Value> = slots
            .into_iter()
            .map(|(start, end)| serde_json::json!({"start": start, "end": end}))
            .collect();

        Ok(ToolResult {
            success: true,
            data: serde_json::json!({"slots": slots_json}),
            message: None,
        })
    }

    async fn create_event(&self, args: Value) -> Result<ToolResult, ToolError> {
        let profile_id = args
            .get("profile_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing profile_id".into()))?;
        let title = args
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing title".into()))?;
        let start = args
            .get("start")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing start".into()))?;
        let end = args
            .get("end")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing end".into()))?;

        let now = Utc::now().to_rfc3339();
        let event = Event {
            id: Uuid::new_v4().to_string(),
            profile_id: profile_id.to_string(),
            title: title.to_string(),
            description: args
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            start_time: start.to_string(),
            end_time: end.to_string(),
            location: args
                .get("location")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            scope: args
                .get("scope")
                .and_then(|v| v.as_str())
                .unwrap_or("shared")
                .to_string(),
            created_at: now.clone(),
            updated_at: now,
        };

        let conn = self
            .db
            .lock()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;
        EventsRepo::create(&conn, &event).map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        Ok(ToolResult {
            success: true,
            data: serde_json::to_value(&event).unwrap_or_default(),
            message: Some("Event created successfully".into()),
        })
    }

    async fn update_event(&self, args: Value) -> Result<ToolResult, ToolError> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing id".into()))?;

        let conn = self
            .db
            .lock()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;
        EventsRepo::update(
            &conn,
            id,
            args.get("title").and_then(|v| v.as_str()),
            args.get("description").and_then(|v| v.as_str()),
            args.get("location").and_then(|v| v.as_str()),
        )
        .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        Ok(ToolResult {
            success: true,
            data: serde_json::json!({"id": id}),
            message: Some("Event updated successfully".into()),
        })
    }
}

#[async_trait]
impl Tool for CalendarTool {
    fn name(&self) -> &'static str {
        "calendar"
    }

    fn description(&self) -> &'static str {
        "Gestión de agenda: eventos, disponibilidad y calendario"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["get_events", "check_availability", "create_event", "update_event"]
                },
                "profile_id": { "type": "string" },
                "date": { "type": "string" },
                "duration": { "type": "integer" },
                "title": { "type": "string" },
                "start": { "type": "string" },
                "end": { "type": "string" },
                "location": { "type": "string" },
                "scope": { "type": "string", "enum": ["shared", "personal"] },
                "id": { "type": "string" },
                "description": { "type": "string" }
            },
            "required": ["operation"]
        })
    }

    fn permission(&self) -> Permission {
        Permission::NoConfirm
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, ToolError> {
        match args.get("operation").and_then(|v| v.as_str()).unwrap_or("") {
            "get_events" => self.get_events(args).await,
            "check_availability" => self.check_availability(args).await,
            "create_event" => self.create_event(args).await,
            "update_event" => self.update_event(args).await,
            op => Err(ToolError::InvalidArguments(format!(
                "Unknown operation: {}",
                op
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::run_migrations;

    fn setup() -> (Arc<Mutex<Connection>>, CalendarTool) {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn.execute(
            "INSERT INTO profiles (id, name, preferences) VALUES ('profile-1', 'Test', '{}')",
            [],
        )
        .unwrap();
        let db = Arc::new(Mutex::new(conn));
        let tool = CalendarTool::new(db.clone());
        (db, tool)
    }

    #[tokio::test]
    async fn test_get_events_empty() {
        let (_, tool) = setup();
        let result = tool
            .execute(serde_json::json!({
                "operation": "get_events",
                "profile_id": "profile-1",
                "start": "2026-09-24T00:00:00Z",
                "end": "2026-09-24T23:59:59Z"
            }))
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.data.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_create_event() {
        let (_, tool) = setup();
        let result = tool
            .execute(serde_json::json!({
                "operation": "create_event",
                "profile_id": "profile-1",
                "title": "Reunión",
                "start": "2026-09-24T10:00:00Z",
                "end": "2026-09-24T11:00:00Z",
                "scope": "shared"
            }))
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.data["title"], "Reunión");
        assert_eq!(result.data["scope"], "shared");
    }

    #[tokio::test]
    async fn test_check_availability() {
        let (_, tool) = setup();
        // First create an event
        tool.execute(serde_json::json!({
            "operation": "create_event",
            "profile_id": "profile-1",
            "title": "Ocupado",
            "start": "2026-09-24T10:00:00Z",
            "end": "2026-09-24T11:00:00Z"
        }))
        .await
        .unwrap();

        let result = tool
            .execute(serde_json::json!({
                "operation": "check_availability",
                "profile_id": "profile-1",
                "date": "2026-09-24",
                "duration": 30
            }))
            .await
            .unwrap();
        assert!(result.success);
        let slots = result.data["slots"].as_array().unwrap();
        assert!(!slots.is_empty());
        // Should have at least one slot before 10:00 and one after 11:00
        assert!(slots.len() >= 2);
    }

    #[tokio::test]
    async fn test_invalid_operation() {
        let (_, tool) = setup();
        let result = tool
            .execute(serde_json::json!({
                "operation": "nonexistent"
            }))
            .await;
        assert!(matches!(result, Err(ToolError::InvalidArguments(_))));
    }

    #[tokio::test]
    async fn test_update_event() {
        let (_, tool) = setup();
        // Create an event first
        let created = tool
            .execute(serde_json::json!({
                "operation": "create_event",
                "profile_id": "profile-1",
                "title": "Original",
                "start": "2026-09-24T10:00:00Z",
                "end": "2026-09-24T11:00:00Z"
            }))
            .await
            .unwrap();
        let event_id = created.data["id"].as_str().unwrap().to_string();

        // Update it
        let result = tool
            .execute(serde_json::json!({
                "operation": "update_event",
                "id": event_id,
                "title": "Actualizado",
                "location": "Oficina"
            }))
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.data["id"], event_id);
    }

    #[tokio::test]
    async fn test_missing_required_args() {
        let (_, tool) = setup();
        let result = tool
            .execute(serde_json::json!({
                "operation": "create_event",
                "profile_id": "profile-1"
                // missing title, start, end
            }))
            .await;
        assert!(matches!(result, Err(ToolError::InvalidArguments(_))));
    }
}
