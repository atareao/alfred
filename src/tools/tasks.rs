use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::db::repos::tasks::{Task, TasksRepo};
use crate::tools::permission::Permission;
use crate::tools::r#trait::{Tool, ToolError, ToolResult};

pub struct TasksTool {
    db: SqlitePool,
}

impl TasksTool {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    async fn list_tasks(&self, args: Value) -> Result<ToolResult, ToolError> {
        let profile_id = args
            .get("profile_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing profile_id".into()))?;

        let tasks = TasksRepo::list(
            &self.db,
            profile_id,
            args.get("status").and_then(|v| v.as_str()),
            args.get("priority").and_then(|v| v.as_str()),
            args.get("project").and_then(|v| v.as_str()),
            args.get("scope").and_then(|v| v.as_str()),
        )
        .await?;

        Ok(ToolResult {
            success: true,
            data: serde_json::to_value(tasks).unwrap_or_default(),
            message: None,
        })
    }

    async fn add_task(&self, args: Value) -> Result<ToolResult, ToolError> {
        let profile_id = args
            .get("profile_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing profile_id".into()))?;
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing content".into()))?;

        let now = Utc::now().to_rfc3339();
        let task = Task {
            id: Uuid::new_v4().to_string(),
            profile_id: profile_id.to_string(),
            content: content.to_string(),
            status: "pending".to_string(),
            priority: args
                .get("priority")
                .and_then(|v| v.as_str())
                .unwrap_or("medium")
                .to_string(),
            project: args
                .get("project")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            due_date: args
                .get("due_date")
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

        TasksRepo::create(&self.db, &task).await?;

        Ok(ToolResult {
            success: true,
            data: serde_json::to_value(&task).unwrap_or_default(),
            message: Some("Task created successfully".into()),
        })
    }

    async fn update_task(&self, args: Value) -> Result<ToolResult, ToolError> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing id".into()))?;

        TasksRepo::update(
            &self.db,
            id,
            args.get("content").and_then(|v| v.as_str()),
            args.get("priority").and_then(|v| v.as_str()),
            args.get("project").and_then(|v| v.as_str()),
            args.get("due_date").and_then(|v| v.as_str()),
            args.get("scope").and_then(|v| v.as_str()),
        )
        .await?;

        Ok(ToolResult {
            success: true,
            data: serde_json::json!({"id": id}),
            message: Some("Task updated successfully".into()),
        })
    }

    async fn complete_task(&self, args: Value) -> Result<ToolResult, ToolError> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing id".into()))?;

        TasksRepo::complete(&self.db, id).await?;

        Ok(ToolResult {
            success: true,
            data: serde_json::json!({"id": id}),
            message: Some("Task completed successfully".into()),
        })
    }
}

#[async_trait]
impl Tool for TasksTool {
    fn name(&self) -> &'static str {
        "tasks"
    }

    fn description(&self) -> &'static str {
        "Gestión de tareas: listar, crear, actualizar y completar tareas"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["list_tasks", "add_task", "update_task", "complete_task"]
                },
                "profile_id": { "type": "string" },
                "content": { "type": "string" },
                "status": { "type": "string", "enum": ["pending", "completed", "cancelled"] },
                "priority": { "type": "string", "enum": ["low", "medium", "high"] },
                "project": { "type": "string" },
                "due_date": { "type": "string" },
                "scope": { "type": "string", "enum": ["shared", "personal"] },
                "id": { "type": "string" }
            },
            "required": ["operation"]
        })
    }

    fn permission(&self) -> Permission {
        Permission::NoConfirm
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, ToolError> {
        match args.get("operation").and_then(|v| v.as_str()).unwrap_or("") {
            "list_tasks" => self.list_tasks(args).await,
            "add_task" => self.add_task(args).await,
            "update_task" => self.update_task(args).await,
            "complete_task" => self.complete_task(args).await,
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
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup() -> Result<(SqlitePool, TasksTool), sqlx::Error> {
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
        let tool = TasksTool::new(pool.clone());
        Ok((pool, tool))
    }

    #[tokio::test]
    async fn test_list_empty() -> Result<(), Box<dyn std::error::Error>> {
        let (_, tool) = setup().await?;
        let result = tool
            .execute(serde_json::json!({
                "operation": "list_tasks",
                "profile_id": "profile-1"
            }))
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.data.as_array().unwrap().len(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn test_add_task() -> Result<(), Box<dyn std::error::Error>> {
        let (_, tool) = setup().await?;
        let result = tool
            .execute(serde_json::json!({
                "operation": "add_task",
                "profile_id": "profile-1",
                "content": "Comprar leche",
                "priority": "high",
                "project": "Casa"
            }))
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.data["content"], "Comprar leche");
        assert_eq!(result.data["priority"], "high");
        assert_eq!(result.data["project"], "Casa");
        assert_eq!(result.data["status"], "pending");
        Ok(())
    }

    #[tokio::test]
    async fn test_complete_task() -> Result<(), Box<dyn std::error::Error>> {
        let (_, tool) = setup().await?;
        // Add a task first
        let created = tool
            .execute(serde_json::json!({
                "operation": "add_task",
                "profile_id": "profile-1",
                "content": "Tarea para completar"
            }))
            .await
            .unwrap();
        let task_id = created.data["id"].as_str().unwrap().to_string();

        // Complete it
        let result = tool
            .execute(serde_json::json!({
                "operation": "complete_task",
                "id": task_id
            }))
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.data["id"], task_id);
        Ok(())
    }

    #[tokio::test]
    async fn test_invalid_operation() -> Result<(), Box<dyn std::error::Error>> {
        let (_, tool) = setup().await?;
        let result = tool
            .execute(serde_json::json!({
                "operation": "nonexistent"
            }))
            .await;
        assert!(matches!(result, Err(ToolError::InvalidArguments(_))));
        Ok(())
    }

    #[tokio::test]
    async fn test_update_task() -> Result<(), Box<dyn std::error::Error>> {
        let (_, tool) = setup().await?;
        // Add a task first
        let created = tool
            .execute(serde_json::json!({
                "operation": "add_task",
                "profile_id": "profile-1",
                "content": "Tarea original",
                "priority": "low"
            }))
            .await
            .unwrap();
        let task_id = created.data["id"].as_str().unwrap().to_string();

        // Update it
        let result = tool
            .execute(serde_json::json!({
                "operation": "update_task",
                "id": task_id,
                "content": "Tarea actualizada",
                "priority": "high"
            }))
            .await
            .unwrap();
        assert!(result.success);

        // Verify via list
        let list_result = tool
            .execute(serde_json::json!({
                "operation": "list_tasks",
                "profile_id": "profile-1"
            }))
            .await
            .unwrap();
        let tasks = list_result.data.as_array().unwrap();
        let updated = tasks.iter().find(|t| t["id"] == task_id).unwrap();
        assert_eq!(updated["content"], "Tarea actualizada");
        assert_eq!(updated["priority"], "high");
        Ok(())
    }

    #[tokio::test]
    async fn test_list_tasks_with_filters() -> Result<(), Box<dyn std::error::Error>> {
        let (_, tool) = setup().await?;
        // Add two tasks with different projects
        tool.execute(serde_json::json!({
            "operation": "add_task",
            "profile_id": "profile-1",
            "content": "Tarea Alpha",
            "project": "Alpha"
        }))
        .await
        .unwrap();

        tool.execute(serde_json::json!({
            "operation": "add_task",
            "profile_id": "profile-1",
            "content": "Tarea Beta",
            "project": "Beta"
        }))
        .await
        .unwrap();

        // Filter by project
        let result = tool
            .execute(serde_json::json!({
                "operation": "list_tasks",
                "profile_id": "profile-1",
                "project": "Alpha"
            }))
            .await
            .unwrap();
        let tasks = result.data.as_array().unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0]["project"], "Alpha");
        Ok(())
    }
}
