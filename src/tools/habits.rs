use async_trait::async_trait;
use chrono::Utc;
use rusqlite::Connection;
use serde_json::Value;
use std::sync::{Arc, Mutex};

use crate::db::repos::habits::HabitsRepo;
use crate::tools::permission::Permission;
use crate::tools::r#trait::{Tool, ToolError, ToolResult};

pub struct HabitsTool {
    db: Arc<Mutex<Connection>>,
}

impl HabitsTool {
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self { db }
    }

    async fn create_habit(&self, args: Value) -> Result<ToolResult, ToolError> {
        let profile_id = args
            .get("profile_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing profile_id".into()))?;
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing name".into()))?;
        let frequency = args
            .get("frequency")
            .and_then(|v| v.as_str())
            .unwrap_or("daily");

        let target = args
            .get("target")
            .and_then(|v| v.as_u64())
            .map(|t| t as u32);

        let conn = self
            .db
            .lock()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        let habit = HabitsRepo::create(&conn, profile_id, name, frequency, target)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        Ok(ToolResult {
            success: true,
            data: serde_json::to_value(&habit).unwrap_or_default(),
            message: Some("Hábito creado exitosamente".into()),
        })
    }

    async fn log_habit(&self, args: Value) -> Result<ToolResult, ToolError> {
        let habit_id = args
            .get("habit_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing habit_id".into()))?;

        let conn = self
            .db
            .lock()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        // Check if the habit exists
        let habit = HabitsRepo::find_by_id(&conn, habit_id)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        if habit.is_none() {
            return Err(ToolError::NotFound(format!(
                "Habit not found: {}",
                habit_id
            )));
        }

        let today = Utc::now().date_naive().to_string();
        HabitsRepo::log(&conn, habit_id, &today)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        Ok(ToolResult {
            success: true,
            data: serde_json::json!({
                "habit_id": habit_id,
                "date": today
            }),
            message: Some("Hábito registrado exitosamente".into()),
        })
    }

    async fn habit_streaks(&self, args: Value) -> Result<ToolResult, ToolError> {
        let profile_id = args
            .get("profile_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing profile_id".into()))?;

        let conn = self
            .db
            .lock()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        let streaks = HabitsRepo::get_streaks(&conn, profile_id)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        Ok(ToolResult {
            success: true,
            data: serde_json::to_value(&streaks).unwrap_or_default(),
            message: None,
        })
    }

    async fn habit_stats(&self, args: Value) -> Result<ToolResult, ToolError> {
        let habit_id = args
            .get("habit_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing habit_id".into()))?;

        let period = args
            .get("period")
            .and_then(|v| v.as_str())
            .unwrap_or("month");

        let conn = self
            .db
            .lock()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        let (total_completed, total_expected) = HabitsRepo::get_stats(&conn, habit_id, period)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        let completion_rate = if total_expected > 0 {
            total_completed as f64 / total_expected as f64
        } else {
            0.0
        };

        Ok(ToolResult {
            success: true,
            data: serde_json::json!({
                "habit_id": habit_id,
                "period": period,
                "completion_rate": completion_rate,
                "total_expected": total_expected,
                "total_completed": total_completed,
            }),
            message: None,
        })
    }
}

#[async_trait]
impl Tool for HabitsTool {
    fn name(&self) -> &'static str {
        "habits"
    }

    fn description(&self) -> &'static str {
        "Seguimiento de hábitos y rachas diarias/semanales"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["create_habit", "log_habit", "habit_streaks", "habit_stats"]
                },
                "profile_id": { "type": "string", "description": "ID del perfil" },
                "name": { "type": "string", "description": "Nombre del hábito" },
                "frequency": {
                    "type": "string",
                    "enum": ["daily", "weekly"],
                    "description": "Frecuencia del hábito (default: daily)"
                },
                "target": { "type": "integer", "description": "Objetivo diario/semanal" },
                "habit_id": { "type": "string", "description": "ID del hábito" },
                "period": {
                    "type": "string",
                    "enum": ["week", "month", "year"],
                    "description": "Período para stats (default: month)"
                }
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
            "create_habit" => self.create_habit(args).await,
            "log_habit" => self.log_habit(args).await,
            "habit_streaks" => self.habit_streaks(args).await,
            "habit_stats" => self.habit_stats(args).await,
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

    fn setup() -> (Arc<Mutex<Connection>>, HabitsTool) {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn.execute(
            "INSERT INTO profiles (id, name, preferences) VALUES ('profile-1', 'Test', '{}')",
            [],
        )
        .unwrap();
        let db = Arc::new(Mutex::new(conn));
        let tool = HabitsTool::new(db.clone());
        (db, tool)
    }

    #[tokio::test]
    async fn test_habits_name_and_description() {
        let (_, tool) = setup();
        assert_eq!(tool.name(), "habits");
        assert_eq!(
            tool.description(),
            "Seguimiento de hábitos y rachas diarias/semanales"
        );
    }

    #[tokio::test]
    async fn test_habits_permission() {
        let (_, tool) = setup();
        assert_eq!(tool.permission(), Permission::Notify);
    }

    #[tokio::test]
    async fn test_habits_parameters_has_operations() {
        let (_, tool) = setup();
        let params = tool.parameters();
        assert_eq!(params["type"], "object");
        let ops = params["properties"]["operation"]["enum"]
            .as_array()
            .unwrap();
        let op_names: Vec<&str> = ops.iter().map(|v| v.as_str().unwrap()).collect();
        assert!(op_names.contains(&"create_habit"));
        assert!(op_names.contains(&"log_habit"));
        assert!(op_names.contains(&"habit_streaks"));
        assert!(op_names.contains(&"habit_stats"));
    }

    #[tokio::test]
    async fn test_create_habit_missing_name() {
        let (_, tool) = setup();
        let err = tool
            .execute(serde_json::json!({
                "operation": "create_habit",
                "profile_id": "profile-1"
            }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidArguments(_)));
    }

    #[tokio::test]
    async fn test_log_habit_not_found() {
        let (_, tool) = setup();
        let err = tool
            .execute(serde_json::json!({
                "operation": "log_habit",
                "habit_id": "nonexistent"
            }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_invalid_operation() {
        let (_, tool) = setup();
        let err = tool
            .execute(serde_json::json!({
                "operation": "nonexistent"
            }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidArguments(_)));
    }

    #[tokio::test]
    async fn test_create_habit_success() {
        let (_, tool) = setup();
        let result = tool
            .execute(serde_json::json!({
                "operation": "create_habit",
                "profile_id": "profile-1",
                "name": "Leer 30 minutos"
            }))
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.data["name"], "Leer 30 minutos");
        assert_eq!(result.data["profile_id"], "profile-1");
        assert_eq!(result.data["frequency"], "daily");
        assert!(result.data["id"].is_string());
    }

    #[tokio::test]
    async fn test_habit_streaks_missing_profile() {
        let (_, tool) = setup();
        let err = tool
            .execute(serde_json::json!({
                "operation": "habit_streaks"
            }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidArguments(_)));
    }

    #[tokio::test]
    async fn test_create_habit_with_custom_frequency() {
        let (_, tool) = setup();
        let result = tool
            .execute(serde_json::json!({
                "operation": "create_habit",
                "profile_id": "profile-1",
                "name": "Jardinería",
                "frequency": "weekly",
                "target": 2
            }))
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.data["name"], "Jardinería");
        assert_eq!(result.data["frequency"], "weekly");
        assert_eq!(result.data["target"], 2);
    }

    #[tokio::test]
    async fn test_log_habit_success() {
        let (_, tool) = setup();
        // Create a habit first
        let created = tool
            .execute(serde_json::json!({
                "operation": "create_habit",
                "profile_id": "profile-1",
                "name": "Meditar"
            }))
            .await
            .unwrap();
        let habit_id = created.data["id"].as_str().unwrap().to_string();

        // Log it
        let result = tool
            .execute(serde_json::json!({
                "operation": "log_habit",
                "habit_id": habit_id
            }))
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.data["habit_id"], habit_id);
        assert!(result.data["date"].is_string());
    }

    #[tokio::test]
    async fn test_habit_streaks_success() {
        let (_, tool) = setup();
        // Create a habit and log it
        let created = tool
            .execute(serde_json::json!({
                "operation": "create_habit",
                "profile_id": "profile-1",
                "name": "Correr"
            }))
            .await
            .unwrap();
        let habit_id = created.data["id"].as_str().unwrap().to_string();

        // Log today
        tool.execute(serde_json::json!({
            "operation": "log_habit",
            "habit_id": habit_id
        }))
        .await
        .unwrap();

        let result = tool
            .execute(serde_json::json!({
                "operation": "habit_streaks",
                "profile_id": "profile-1"
            }))
            .await
            .unwrap();
        assert!(result.success);
        let streaks = result.data.as_array().unwrap();
        assert_eq!(streaks.len(), 1);
        assert_eq!(streaks[0]["habit_id"], habit_id);
    }

    #[tokio::test]
    async fn test_habit_stats_success() {
        let (_, tool) = setup();
        // Create a habit
        let created = tool
            .execute(serde_json::json!({
                "operation": "create_habit",
                "profile_id": "profile-1",
                "name": "Agua"
            }))
            .await
            .unwrap();
        let habit_id = created.data["id"].as_str().unwrap().to_string();

        // Log it today
        tool.execute(serde_json::json!({
            "operation": "log_habit",
            "habit_id": habit_id
        }))
        .await
        .unwrap();

        // Get stats
        let result = tool
            .execute(serde_json::json!({
                "operation": "habit_stats",
                "habit_id": habit_id,
                "period": "week"
            }))
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.data["habit_id"], habit_id);
        assert_eq!(result.data["period"], "week");
        assert!(result.data["completion_rate"].is_f64());
        assert!(result.data["total_expected"].is_u64());
        assert!(result.data["total_completed"].is_u64());
    }

    #[tokio::test]
    async fn test_habit_stats_default_period() {
        let (_, tool) = setup();
        let created = tool
            .execute(serde_json::json!({
                "operation": "create_habit",
                "profile_id": "profile-1",
                "name": "Ejercicio"
            }))
            .await
            .unwrap();
        let habit_id = created.data["id"].as_str().unwrap().to_string();

        // No period specified — should default to "month"
        let result = tool
            .execute(serde_json::json!({
                "operation": "habit_stats",
                "habit_id": habit_id
            }))
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.data["period"], "month");
    }
}
