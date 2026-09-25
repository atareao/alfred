use async_trait::async_trait;
use chrono::{Duration, Utc};
use serde_json::Value;
use sqlx::SqlitePool;

use crate::db::repos::reminders::{Reminder, RemindersRepo};
use crate::tools::permission::Permission;
use crate::tools::r#trait::{Tool, ToolError, ToolResult};

pub struct RemindersTool {
    db: SqlitePool,
}

impl RemindersTool {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    async fn set_reminder(&self, args: Value) -> Result<ToolResult, ToolError> {
        let profile_id = args
            .get("profile_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();
        let text = args
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let datetime = args
            .get("datetime")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if text.is_empty() || datetime.is_empty() {
            return Err(ToolError::InvalidArguments(
                "text and datetime are required".into(),
            ));
        }

        let reminder = Reminder {
            id: uuid::Uuid::new_v4().to_string(),
            profile_id,
            text,
            datetime,
            status: "pending".into(),
            created_at: Utc::now().to_rfc3339(),
        };

        RemindersRepo::create(&self.db, &reminder).await?;

        Ok(ToolResult {
            success: true,
            data: serde_json::to_value(&reminder).unwrap_or_default(),
            message: Some("Reminder set".into()),
        })
    }

    async fn list_reminders(&self, args: Value) -> Result<ToolResult, ToolError> {
        let profile_id = args
            .get("profile_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default");
        let status = args.get("status").and_then(|v| v.as_str());

        let reminders = RemindersRepo::list(&self.db, profile_id, status).await?;

        Ok(ToolResult {
            success: true,
            data: serde_json::to_value(&reminders).unwrap_or_default(),
            message: Some(format!("Found {} reminders", reminders.len())),
        })
    }

    async fn dismiss_reminder(&self, args: Value) -> Result<ToolResult, ToolError> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if id.is_empty() {
            return Err(ToolError::InvalidArguments("id is required".into()));
        }

        RemindersRepo::dismiss(&self.db, &id).await?;

        Ok(ToolResult {
            success: true,
            data: serde_json::json!({"id": id}),
            message: Some("Reminder dismissed".into()),
        })
    }

    async fn snooze_reminder(&self, args: Value) -> Result<ToolResult, ToolError> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let minutes = args.get("minutes").and_then(|v| v.as_i64()).unwrap_or(10);

        if id.is_empty() {
            return Err(ToolError::InvalidArguments("id is required".into()));
        }

        // Compute new datetime: now + minutes
        let new_datetime = Utc::now() + Duration::minutes(minutes);
        let new_datetime_str = new_datetime.to_rfc3339();

        RemindersRepo::snooze(&self.db, &id, &new_datetime_str).await?;

        Ok(ToolResult {
            success: true,
            data: serde_json::json!({"id": id, "snoozed_minutes": minutes, "new_datetime": new_datetime_str}),
            message: Some("Reminder snoozed".into()),
        })
    }
}

#[async_trait]
impl Tool for RemindersTool {
    fn name(&self) -> &'static str {
        "reminders"
    }

    fn description(&self) -> &'static str {
        "Gestión de recordatorios con alarma"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["set_reminder", "list_reminders", "dismiss_reminder", "snooze_reminder"]
                },
                "profile_id": { "type": "string", "description": "Profile ID (defaults to 'default')" },
                "text": { "type": "string", "description": "Reminder text" },
                "datetime": { "type": "string", "description": "ISO 8601 datetime" },
                "id": { "type": "string", "description": "Reminder ID" },
                "minutes": { "type": "integer", "description": "Minutes to snooze" },
                "status": { "type": "string", "enum": ["pending", "dismissed", "snoozed"] }
            },
            "required": ["operation"]
        })
    }

    fn permission(&self) -> Permission {
        Permission::Notify
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, ToolError> {
        let operation = args.get("operation").and_then(|v| v.as_str()).unwrap_or("");

        match operation {
            "set_reminder" => self.set_reminder(args).await,
            "list_reminders" => self.list_reminders(args).await,
            "dismiss_reminder" => self.dismiss_reminder(args).await,
            "snooze_reminder" => self.snooze_reminder(args).await,
            _ => Err(ToolError::InvalidArguments(format!(
                "Unknown operation: {}",
                operation
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::run_migrations;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_db() -> Result<SqlitePool, sqlx::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;
        run_migrations(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO profiles (id, name, preferences) VALUES ('profile-1', 'Test', '{}')",
        )
        .execute(&pool)
        .await?;
        Ok(pool)
    }

    #[tokio::test]
    async fn test_reminders_name_and_description() -> Result<(), Box<dyn std::error::Error>> {
        let db = setup_db().await?;
        let tool = RemindersTool::new(db);
        assert_eq!(tool.name(), "reminders");
        assert_eq!(tool.description(), "Gestión de recordatorios con alarma");
        Ok(())
    }

    #[tokio::test]
    async fn test_reminders_permission() -> Result<(), Box<dyn std::error::Error>> {
        let db = setup_db().await?;
        let tool = RemindersTool::new(db);
        assert_eq!(tool.permission(), Permission::Notify);
        Ok(())
    }

    #[tokio::test]
    async fn test_set_reminder() -> Result<(), Box<dyn std::error::Error>> {
        let db = setup_db().await?;
        let tool = RemindersTool::new(db);

        let args = serde_json::json!({
            "operation": "set_reminder",
            "profile_id": "profile-1",
            "text": "Comprar leche",
            "datetime": "2026-09-24T10:00:00Z"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.success);
        assert_eq!(result.data["text"], "Comprar leche");
        assert_eq!(result.data["status"], "pending");
        assert!(result.data["id"].is_string());
        Ok(())
    }

    #[tokio::test]
    async fn test_set_reminder_missing_fields() -> Result<(), Box<dyn std::error::Error>> {
        let db = setup_db().await?;
        let tool = RemindersTool::new(db);

        let args = serde_json::json!({
            "operation": "set_reminder",
            "text": "Solo texto"
        });

        let err = tool.execute(args).await.unwrap_err();
        assert!(matches!(err, ToolError::InvalidArguments(_)));
        Ok(())
    }

    #[tokio::test]
    async fn test_list_reminders() -> Result<(), Box<dyn std::error::Error>> {
        let db = setup_db().await?;
        let tool = RemindersTool::new(db);

        // Create two reminders first
        tool.execute(serde_json::json!({
            "operation": "set_reminder",
            "profile_id": "profile-1",
            "text": "Recordatorio 1",
            "datetime": "2026-09-24T10:00:00Z"
        }))
        .await
        .unwrap();

        tool.execute(serde_json::json!({
            "operation": "set_reminder",
            "profile_id": "profile-1",
            "text": "Recordatorio 2",
            "datetime": "2026-09-24T11:00:00Z"
        }))
        .await
        .unwrap();

        let result = tool
            .execute(serde_json::json!({
                "operation": "list_reminders",
                "profile_id": "profile-1"
            }))
            .await
            .unwrap();

        assert!(result.success);
        let reminders: Vec<Reminder> = serde_json::from_value(result.data).unwrap();
        assert_eq!(reminders.len(), 2);
        Ok(())
    }

    #[tokio::test]
    async fn test_list_reminders_by_status() -> Result<(), Box<dyn std::error::Error>> {
        let db = setup_db().await?;
        let tool = RemindersTool::new(db);

        let r1 = tool
            .execute(serde_json::json!({
                "operation": "set_reminder",
                "profile_id": "profile-1",
                "text": "Pendiente",
                "datetime": "2026-09-24T10:00:00Z"
            }))
            .await
            .unwrap();
        let id1 = r1.data["id"].as_str().unwrap().to_string();

        let r2 = tool
            .execute(serde_json::json!({
                "operation": "set_reminder",
                "profile_id": "profile-1",
                "text": "A dismiss",
                "datetime": "2026-09-24T11:00:00Z"
            }))
            .await
            .unwrap();
        let id2 = r2.data["id"].as_str().unwrap().to_string();

        // Dismiss the second one
        tool.execute(serde_json::json!({
            "operation": "dismiss_reminder",
            "id": id2
        }))
        .await
        .unwrap();

        // List only pending
        let pending = tool
            .execute(serde_json::json!({
                "operation": "list_reminders",
                "profile_id": "profile-1",
                "status": "pending"
            }))
            .await
            .unwrap();
        let pending_list: Vec<Reminder> = serde_json::from_value(pending.data).unwrap();
        assert_eq!(pending_list.len(), 1);
        assert_eq!(pending_list[0].id, id1);

        // List only dismissed
        let dismissed = tool
            .execute(serde_json::json!({
                "operation": "list_reminders",
                "profile_id": "profile-1",
                "status": "dismissed"
            }))
            .await
            .unwrap();
        let dismissed_list: Vec<Reminder> = serde_json::from_value(dismissed.data).unwrap();
        assert_eq!(dismissed_list.len(), 1);
        assert_eq!(dismissed_list[0].id, id2);
        Ok(())
    }

    #[tokio::test]
    async fn test_dismiss_reminder() -> Result<(), Box<dyn std::error::Error>> {
        let db = setup_db().await?;
        let tool = RemindersTool::new(db);

        let created = tool
            .execute(serde_json::json!({
                "operation": "set_reminder",
                "profile_id": "profile-1",
                "text": "Para dismiss",
                "datetime": "2026-09-24T10:00:00Z"
            }))
            .await
            .unwrap();
        let id = created.data["id"].as_str().unwrap().to_string();

        let result = tool
            .execute(serde_json::json!({
                "operation": "dismiss_reminder",
                "id": id
            }))
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.data["id"], id);

        // Verify it's dismissed by listing
        let pending = tool
            .execute(serde_json::json!({
                "operation": "list_reminders",
                "profile_id": "profile-1",
                "status": "dismissed"
            }))
            .await
            .unwrap();
        let list: Vec<Reminder> = serde_json::from_value(pending.data).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, id);
        Ok(())
    }

    #[tokio::test]
    async fn test_dismiss_reminder_missing_id() -> Result<(), Box<dyn std::error::Error>> {
        let db = setup_db().await?;
        let tool = RemindersTool::new(db);

        let err = tool
            .execute(serde_json::json!({
                "operation": "dismiss_reminder"
            }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidArguments(_)));
        Ok(())
    }

    #[tokio::test]
    async fn test_snooze_reminder() -> Result<(), Box<dyn std::error::Error>> {
        let db = setup_db().await?;
        let tool = RemindersTool::new(db);

        let created = tool
            .execute(serde_json::json!({
                "operation": "set_reminder",
                "profile_id": "profile-1",
                "text": "Para snooze",
                "datetime": "2026-09-24T10:00:00Z"
            }))
            .await
            .unwrap();
        let id = created.data["id"].as_str().unwrap().to_string();

        let result = tool
            .execute(serde_json::json!({
                "operation": "snooze_reminder",
                "id": id,
                "minutes": 15
            }))
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.data["id"], id);
        assert_eq!(result.data["snoozed_minutes"], 15);
        assert!(result.data["new_datetime"].is_string());
        Ok(())
    }

    #[tokio::test]
    async fn test_snooze_reminder_default_minutes() -> Result<(), Box<dyn std::error::Error>> {
        let db = setup_db().await?;
        let tool = RemindersTool::new(db);

        let created = tool
            .execute(serde_json::json!({
                "operation": "set_reminder",
                "profile_id": "profile-1",
                "text": "Snooze default",
                "datetime": "2026-09-24T10:00:00Z"
            }))
            .await
            .unwrap();
        let id = created.data["id"].as_str().unwrap().to_string();

        let result = tool
            .execute(serde_json::json!({
                "operation": "snooze_reminder",
                "id": id
            }))
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.data["snoozed_minutes"], 10);
        Ok(())
    }

    #[tokio::test]
    async fn test_snooze_reminder_missing_id() -> Result<(), Box<dyn std::error::Error>> {
        let db = setup_db().await?;
        let tool = RemindersTool::new(db);

        let err = tool
            .execute(serde_json::json!({
                "operation": "snooze_reminder",
                "minutes": 5
            }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidArguments(_)));
        Ok(())
    }

    #[tokio::test]
    async fn test_unknown_operation() -> Result<(), Box<dyn std::error::Error>> {
        let db = setup_db().await?;
        let tool = RemindersTool::new(db);

        let err = tool
            .execute(serde_json::json!({
                "operation": "nonexistent"
            }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidArguments(_)));
        Ok(())
    }

    #[tokio::test]
    async fn test_parameters_returns_valid_json_schema() -> Result<(), Box<dyn std::error::Error>> {
        let db = setup_db().await?;
        let tool = RemindersTool::new(db);
        let params = tool.parameters();
        assert_eq!(params["type"], "object");
        assert!(params.get("properties").is_some());
        assert!(params.get("required").is_some());
        assert_eq!(params["required"][0], "operation");
        Ok(())
    }
}
