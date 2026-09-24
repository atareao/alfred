use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::db::repos::notes::{Note, NotesRepo};
use crate::tools::permission::Permission;
use crate::tools::r#trait::{Tool, ToolError, ToolResult};

pub struct NotesTool {
    db: Arc<Mutex<Connection>>,
}

impl NotesTool {
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self { db }
    }

    async fn create_note(&self, args: Value) -> Result<ToolResult, ToolError> {
        let conn = self
            .db
            .lock()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        let profile_id = args
            .get("profile_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let category = args
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("idea")
            .to_string();
        let tags = args
            .get("tags")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if content.is_empty() {
            return Err(ToolError::InvalidArguments("content is required".into()));
        }

        let now = Utc::now().to_rfc3339();
        let note = Note {
            id: uuid::Uuid::new_v4().to_string(),
            profile_id,
            content,
            category,
            tags,
            created_at: now.clone(),
            updated_at: now,
        };

        NotesRepo::create(&conn, &note).map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        Ok(ToolResult {
            success: true,
            data: serde_json::to_value(&note).unwrap_or_default(),
            message: Some("Note created".into()),
        })
    }

    async fn list_notes(&self, args: Value) -> Result<ToolResult, ToolError> {
        let conn = self
            .db
            .lock()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        let profile_id = args
            .get("profile_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default");
        let category = args.get("category").and_then(|v| v.as_str());

        let notes = NotesRepo::list(&conn, profile_id, category)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        Ok(ToolResult {
            success: true,
            data: serde_json::to_value(&notes).unwrap_or_default(),
            message: Some(format!("Found {} notes", notes.len())),
        })
    }

    async fn delete_note(&self, args: Value) -> Result<ToolResult, ToolError> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if id.is_empty() {
            return Err(ToolError::InvalidArguments("id is required".into()));
        }

        let conn = self
            .db
            .lock()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        NotesRepo::delete(&conn, &id).map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        Ok(ToolResult {
            success: true,
            data: serde_json::json!({"id": id}),
            message: Some("Note deleted".into()),
        })
    }
}

#[async_trait]
impl Tool for NotesTool {
    fn name(&self) -> &'static str {
        "notes"
    }

    fn description(&self) -> &'static str {
        "Gestión de notas personales con categorías (idea, journal, fact, todo)"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["create_note", "list_notes", "delete_note"]
                },
                "profile_id": { "type": "string", "description": "Profile ID (defaults to 'default')" },
                "content": { "type": "string", "description": "Note content" },
                "category": {
                    "type": "string",
                    "enum": ["idea", "journal", "fact", "todo"],
                    "description": "Note category (defaults to 'idea')"
                },
                "tags": { "type": "string", "description": "Comma-separated tags" },
                "id": { "type": "string", "description": "Note ID" }
            },
            "required": ["operation"]
        })
    }

    fn permission(&self) -> Permission {
        Permission::NoConfirm
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, ToolError> {
        let operation = args.get("operation").and_then(|v| v.as_str()).unwrap_or("");

        match operation {
            "create_note" => self.create_note(args).await,
            "list_notes" => self.list_notes(args).await,
            "delete_note" => self.delete_note(args).await,
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

    fn setup_db() -> Arc<Mutex<Connection>> {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn.execute(
            "INSERT INTO profiles (id, name, preferences) VALUES ('profile-1', 'Test', '{}')",
            [],
        )
        .unwrap();
        Arc::new(Mutex::new(conn))
    }

    #[tokio::test]
    async fn test_notes_name_and_description() {
        let db = setup_db();
        let tool = NotesTool::new(db);
        assert_eq!(tool.name(), "notes");
        assert!(tool.description().contains("categorías"));
    }

    #[tokio::test]
    async fn test_notes_permission() {
        let db = setup_db();
        let tool = NotesTool::new(db);
        assert_eq!(tool.permission(), Permission::NoConfirm);
    }

    #[tokio::test]
    async fn test_create_note_idea() {
        let db = setup_db();
        let tool = NotesTool::new(db);

        let args = serde_json::json!({
            "operation": "create_note",
            "profile_id": "profile-1",
            "content": "Una idea genial",
            "category": "idea",
            "tags": "creatividad,proyecto"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.success);
        assert_eq!(result.data["content"], "Una idea genial");
        assert_eq!(result.data["category"], "idea");
        assert_eq!(result.data["tags"], "creatividad,proyecto");
    }

    #[tokio::test]
    async fn test_create_note_journal() {
        let db = setup_db();
        let tool = NotesTool::new(db);

        let args = serde_json::json!({
            "operation": "create_note",
            "profile_id": "profile-1",
            "content": "Hoy fue un gran día",
            "category": "journal"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.success);
        assert_eq!(result.data["category"], "journal");
    }

    #[tokio::test]
    async fn test_create_note_fact() {
        let db = setup_db();
        let tool = NotesTool::new(db);

        let args = serde_json::json!({
            "operation": "create_note",
            "profile_id": "profile-1",
            "content": "La velocidad de la luz es 299.792.458 m/s",
            "category": "fact"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.success);
        assert_eq!(result.data["category"], "fact");
    }

    #[tokio::test]
    async fn test_create_note_todo() {
        let db = setup_db();
        let tool = NotesTool::new(db);

        let args = serde_json::json!({
            "operation": "create_note",
            "profile_id": "profile-1",
            "content": "Comprar regalos de Navidad",
            "category": "todo"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.success);
        assert_eq!(result.data["category"], "todo");
    }

    #[tokio::test]
    async fn test_create_note_default_category() {
        let db = setup_db();
        let tool = NotesTool::new(db);

        let args = serde_json::json!({
            "operation": "create_note",
            "profile_id": "profile-1",
            "content": "Nota sin categoría explícita"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.success);
        assert_eq!(result.data["category"], "idea");
    }

    #[tokio::test]
    async fn test_create_note_missing_content() {
        let db = setup_db();
        let tool = NotesTool::new(db);

        let err = tool
            .execute(serde_json::json!({
                "operation": "create_note",
                "category": "idea"
            }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidArguments(_)));
    }

    #[tokio::test]
    async fn test_list_notes() {
        let db = setup_db();
        let tool = NotesTool::new(db);

        // Create notes in different categories
        tool.execute(serde_json::json!({
            "operation": "create_note",
            "profile_id": "profile-1",
            "content": "Idea 1",
            "category": "idea"
        }))
        .await
        .unwrap();

        tool.execute(serde_json::json!({
            "operation": "create_note",
            "profile_id": "profile-1",
            "content": "Journal entry",
            "category": "journal"
        }))
        .await
        .unwrap();

        tool.execute(serde_json::json!({
            "operation": "create_note",
            "profile_id": "profile-1",
            "content": "Fact 1",
            "category": "fact"
        }))
        .await
        .unwrap();

        // List all
        let result = tool
            .execute(serde_json::json!({
                "operation": "list_notes",
                "profile_id": "profile-1"
            }))
            .await
            .unwrap();
        let all: Vec<Note> = serde_json::from_value(result.data).unwrap();
        assert_eq!(all.len(), 3);
    }

    #[tokio::test]
    async fn test_list_notes_by_category() {
        let db = setup_db();
        let tool = NotesTool::new(db);

        tool.execute(serde_json::json!({
            "operation": "create_note",
            "profile_id": "profile-1",
            "content": "Idea A",
            "category": "idea"
        }))
        .await
        .unwrap();

        tool.execute(serde_json::json!({
            "operation": "create_note",
            "profile_id": "profile-1",
            "content": "Idea B",
            "category": "idea"
        }))
        .await
        .unwrap();

        tool.execute(serde_json::json!({
            "operation": "create_note",
            "profile_id": "profile-1",
            "content": "Todo item",
            "category": "todo"
        }))
        .await
        .unwrap();

        let result = tool
            .execute(serde_json::json!({
                "operation": "list_notes",
                "profile_id": "profile-1",
                "category": "idea"
            }))
            .await
            .unwrap();
        let ideas: Vec<Note> = serde_json::from_value(result.data).unwrap();
        assert_eq!(ideas.len(), 2);

        let result = tool
            .execute(serde_json::json!({
                "operation": "list_notes",
                "profile_id": "profile-1",
                "category": "todo"
            }))
            .await
            .unwrap();
        let todos: Vec<Note> = serde_json::from_value(result.data).unwrap();
        assert_eq!(todos.len(), 1);
    }

    #[tokio::test]
    async fn test_delete_note() {
        let db = setup_db();
        let tool = NotesTool::new(db);

        let created = tool
            .execute(serde_json::json!({
                "operation": "create_note",
                "profile_id": "profile-1",
                "content": "Para borrar",
                "category": "todo"
            }))
            .await
            .unwrap();
        let id = created.data["id"].as_str().unwrap().to_string();

        let result = tool
            .execute(serde_json::json!({
                "operation": "delete_note",
                "id": id
            }))
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.data["id"], id);

        // Verify it's gone
        let list = tool
            .execute(serde_json::json!({
                "operation": "list_notes",
                "profile_id": "profile-1"
            }))
            .await
            .unwrap();
        let notes: Vec<Note> = serde_json::from_value(list.data).unwrap();
        assert!(notes.is_empty());
    }

    #[tokio::test]
    async fn test_delete_note_missing_id() {
        let db = setup_db();
        let tool = NotesTool::new(db);

        let err = tool
            .execute(serde_json::json!({
                "operation": "delete_note"
            }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidArguments(_)));
    }

    #[tokio::test]
    async fn test_unknown_operation() {
        let db = setup_db();
        let tool = NotesTool::new(db);

        let err = tool
            .execute(serde_json::json!({
                "operation": "nope"
            }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidArguments(_)));
    }

    #[tokio::test]
    async fn test_parameters_returns_valid_json_schema() {
        let db = setup_db();
        let tool = NotesTool::new(db);
        let params = tool.parameters();
        assert_eq!(params["type"], "object");
        assert!(params.get("properties").is_some());
        assert!(params.get("required").is_some());
    }
}
