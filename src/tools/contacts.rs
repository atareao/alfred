use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::db::repos::contacts::{Contact, ContactsRepo};
use crate::tools::permission::Permission;
use crate::tools::r#trait::{Tool, ToolError, ToolResult};

pub struct ContactsTool {
    db: Arc<Mutex<Connection>>,
}

impl ContactsTool {
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self { db }
    }

    async fn add_contact(&self, args: Value) -> Result<ToolResult, ToolError> {
        let conn = self
            .db
            .lock()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        let profile_id = args
            .get("profile_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let phone = args
            .get("phone")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let email = args
            .get("email")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let notes = args
            .get("notes")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if name.is_empty() {
            return Err(ToolError::InvalidArguments("name is required".into()));
        }

        let now = Utc::now().to_rfc3339();
        let contact = Contact {
            id: uuid::Uuid::new_v4().to_string(),
            profile_id,
            name,
            phone,
            email,
            notes,
            created_at: now.clone(),
            updated_at: now,
        };

        ContactsRepo::create(&conn, &contact)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        Ok(ToolResult {
            success: true,
            data: serde_json::to_value(&contact).unwrap_or_default(),
            message: Some("Contact added".into()),
        })
    }

    async fn search_contacts(&self, args: Value) -> Result<ToolResult, ToolError> {
        let conn = self
            .db
            .lock()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        let profile_id = args
            .get("profile_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default");
        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");

        if query.is_empty() {
            return Err(ToolError::InvalidArguments("query is required".into()));
        }

        let contacts = ContactsRepo::search(&conn, profile_id, query)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        Ok(ToolResult {
            success: true,
            data: serde_json::to_value(&contacts).unwrap_or_default(),
            message: Some(format!("Found {} contacts", contacts.len())),
        })
    }

    async fn update_contact(&self, args: Value) -> Result<ToolResult, ToolError> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let name = args.get("name").and_then(|v| v.as_str());
        let phone = args.get("phone").and_then(|v| v.as_str());
        let email = args.get("email").and_then(|v| v.as_str());
        let notes = args.get("notes").and_then(|v| v.as_str());

        if id.is_empty() {
            return Err(ToolError::InvalidArguments("id is required".into()));
        }

        if name.or(phone).or(email).or(notes).is_none() {
            return Err(ToolError::InvalidArguments(
                "at least one field (name, phone, email, notes) must be provided".into(),
            ));
        }

        let conn = self
            .db
            .lock()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        ContactsRepo::update(&conn, &id, name, phone, email, notes)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        // Fetch the updated contact to return
        let updated = ContactsRepo::find_by_id(&conn, &id)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?
            .ok_or_else(|| ToolError::NotFound(format!("Contact {} not found", id)))?;

        Ok(ToolResult {
            success: true,
            data: serde_json::to_value(&updated).unwrap_or_default(),
            message: Some("Contact updated".into()),
        })
    }
}

#[async_trait]
impl Tool for ContactsTool {
    fn name(&self) -> &'static str {
        "contacts"
    }

    fn description(&self) -> &'static str {
        "Gestión de contactos: buscar, añadir y actualizar información de contacto"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["add_contact", "search_contacts", "update_contact"]
                },
                "profile_id": { "type": "string", "description": "Profile ID (defaults to 'default')" },
                "name": { "type": "string", "description": "Contact name" },
                "phone": { "type": "string", "description": "Phone number" },
                "email": { "type": "string", "description": "Email address" },
                "notes": { "type": "string", "description": "Additional notes" },
                "query": { "type": "string", "description": "Search query (matches name, phone, or email)" },
                "id": { "type": "string", "description": "Contact ID" }
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
            "add_contact" => self.add_contact(args).await,
            "search_contacts" => self.search_contacts(args).await,
            "update_contact" => self.update_contact(args).await,
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
    async fn test_contacts_name_and_description() {
        let db = setup_db();
        let tool = ContactsTool::new(db);
        assert_eq!(tool.name(), "contacts");
        assert!(tool.description().contains("contactos"));
    }

    #[tokio::test]
    async fn test_contacts_permission() {
        let db = setup_db();
        let tool = ContactsTool::new(db);
        assert_eq!(tool.permission(), Permission::NoConfirm);
    }

    #[tokio::test]
    async fn test_add_contact() {
        let db = setup_db();
        let tool = ContactsTool::new(db);

        let args = serde_json::json!({
            "operation": "add_contact",
            "profile_id": "profile-1",
            "name": "Juan Pérez",
            "phone": "+34 600 000 000",
            "email": "juan@example.com"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.success);
        assert_eq!(result.data["name"], "Juan Pérez");
        assert_eq!(result.data["phone"], "+34 600 000 000");
        assert_eq!(result.data["email"], "juan@example.com");
        assert!(result.data["id"].is_string());
    }

    #[tokio::test]
    async fn test_add_contact_minimal() {
        let db = setup_db();
        let tool = ContactsTool::new(db);

        let args = serde_json::json!({
            "operation": "add_contact",
            "profile_id": "profile-1",
            "name": "Solo Nombre"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.success);
        assert_eq!(result.data["name"], "Solo Nombre");
    }

    #[tokio::test]
    async fn test_add_contact_missing_name() {
        let db = setup_db();
        let tool = ContactsTool::new(db);

        let err = tool
            .execute(serde_json::json!({
                "operation": "add_contact",
                "phone": "+34 600"
            }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidArguments(_)));
    }

    #[tokio::test]
    async fn test_search_contacts_by_name() {
        let db = setup_db();
        let tool = ContactsTool::new(db);

        tool.execute(serde_json::json!({
            "operation": "add_contact",
            "profile_id": "profile-1",
            "name": "María García"
        }))
        .await
        .unwrap();

        tool.execute(serde_json::json!({
            "operation": "add_contact",
            "profile_id": "profile-1",
            "name": "Carlos López"
        }))
        .await
        .unwrap();

        tool.execute(serde_json::json!({
            "operation": "add_contact",
            "profile_id": "profile-1",
            "name": "Ana Martínez"
        }))
        .await
        .unwrap();

        let result = tool
            .execute(serde_json::json!({
                "operation": "search_contacts",
                "profile_id": "profile-1",
                "query": "María"
            }))
            .await
            .unwrap();

        assert!(result.success);
        let contacts: Vec<Contact> = serde_json::from_value(result.data).unwrap();
        assert_eq!(contacts.len(), 1);
        assert_eq!(contacts[0].name, "María García");
    }

    #[tokio::test]
    async fn test_search_contacts_by_phone() {
        let db = setup_db();
        let tool = ContactsTool::new(db);

        tool.execute(serde_json::json!({
            "operation": "add_contact",
            "profile_id": "profile-1",
            "name": "Pedro",
            "phone": "+34 611 111 111"
        }))
        .await
        .unwrap();

        let result = tool
            .execute(serde_json::json!({
                "operation": "search_contacts",
                "profile_id": "profile-1",
                "query": "611"
            }))
            .await
            .unwrap();

        assert!(result.success);
        let contacts: Vec<Contact> = serde_json::from_value(result.data).unwrap();
        assert_eq!(contacts.len(), 1);
        assert_eq!(contacts[0].name, "Pedro");
    }

    #[tokio::test]
    async fn test_search_contacts_by_email() {
        let db = setup_db();
        let tool = ContactsTool::new(db);

        tool.execute(serde_json::json!({
            "operation": "add_contact",
            "profile_id": "profile-1",
            "name": "Luis",
            "email": "luis@work.com"
        }))
        .await
        .unwrap();

        let result = tool
            .execute(serde_json::json!({
                "operation": "search_contacts",
                "profile_id": "profile-1",
                "query": "work"
            }))
            .await
            .unwrap();

        assert!(result.success);
        let contacts: Vec<Contact> = serde_json::from_value(result.data).unwrap();
        assert_eq!(contacts.len(), 1);
        assert_eq!(contacts[0].name, "Luis");
    }

    #[tokio::test]
    async fn test_search_contacts_no_results() {
        let db = setup_db();
        let tool = ContactsTool::new(db);

        tool.execute(serde_json::json!({
            "operation": "add_contact",
            "profile_id": "profile-1",
            "name": "Solo yo"
        }))
        .await
        .unwrap();

        let result = tool
            .execute(serde_json::json!({
                "operation": "search_contacts",
                "profile_id": "profile-1",
                "query": "ZzzNadie"
            }))
            .await
            .unwrap();

        assert!(result.success);
        let contacts: Vec<Contact> = serde_json::from_value(result.data).unwrap();
        assert!(contacts.is_empty());
    }

    #[tokio::test]
    async fn test_search_contacts_empty_query() {
        let db = setup_db();
        let tool = ContactsTool::new(db);

        let err = tool
            .execute(serde_json::json!({
                "operation": "search_contacts",
                "profile_id": "profile-1",
                "query": ""
            }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidArguments(_)));
    }

    #[tokio::test]
    async fn test_update_contact() {
        let db = setup_db();
        let tool = ContactsTool::new(db);

        let created = tool
            .execute(serde_json::json!({
                "operation": "add_contact",
                "profile_id": "profile-1",
                "name": "Nombre Original"
            }))
            .await
            .unwrap();
        let id = created.data["id"].as_str().unwrap().to_string();

        let result = tool
            .execute(serde_json::json!({
                "operation": "update_contact",
                "id": id,
                "name": "Nombre Nuevo",
                "phone": "+34 600 000 001",
                "notes": "Nota importante"
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.data["name"], "Nombre Nuevo");
        assert_eq!(result.data["phone"], "+34 600 000 001");
        assert_eq!(result.data["notes"], "Nota importante");
    }

    #[tokio::test]
    async fn test_update_contact_partial() {
        let db = setup_db();
        let tool = ContactsTool::new(db);

        let created = tool
            .execute(serde_json::json!({
                "operation": "add_contact",
                "profile_id": "profile-1",
                "name": "Ana",
                "email": "ana@example.com"
            }))
            .await
            .unwrap();
        let id = created.data["id"].as_str().unwrap().to_string();

        // Update only the phone
        let result = tool
            .execute(serde_json::json!({
                "operation": "update_contact",
                "id": id,
                "phone": "+34 600 000 002"
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.data["phone"], "+34 600 000 002");
        assert_eq!(result.data["email"], "ana@example.com"); // unchanged
    }

    #[tokio::test]
    async fn test_update_contact_missing_id() {
        let db = setup_db();
        let tool = ContactsTool::new(db);

        let err = tool
            .execute(serde_json::json!({
                "operation": "update_contact",
                "name": "No tengo ID"
            }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidArguments(_)));
    }

    #[tokio::test]
    async fn test_update_contact_no_fields() {
        let db = setup_db();
        let tool = ContactsTool::new(db);

        let err = tool
            .execute(serde_json::json!({
                "operation": "update_contact",
                "id": "some-id"
            }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidArguments(_)));
    }

    #[tokio::test]
    async fn test_update_contact_not_found() {
        let db = setup_db();
        let tool = ContactsTool::new(db);

        let err = tool
            .execute(serde_json::json!({
                "operation": "update_contact",
                "id": "nonexistent-id",
                "name": "New Name"
            }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_unknown_operation() {
        let db = setup_db();
        let tool = ContactsTool::new(db);

        let err = tool
            .execute(serde_json::json!({
                "operation": "unknown_op"
            }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidArguments(_)));
    }

    #[tokio::test]
    async fn test_parameters_returns_valid_json_schema() {
        let db = setup_db();
        let tool = ContactsTool::new(db);
        let params = tool.parameters();
        assert_eq!(params["type"], "object");
        assert!(params.get("properties").is_some());
        assert!(params.get("required").is_some());
    }
}
