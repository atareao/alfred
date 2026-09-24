use crate::tools::permission::Permission;
use crate::tools::r#trait::{Tool, ToolError, ToolResult};
use async_trait::async_trait;
use rusqlite::Connection;
use serde_json::Value;
use std::sync::{Arc, Mutex};

pub struct UnifiedSearchTool {
    db: Arc<Mutex<Connection>>,
}

impl UnifiedSearchTool {
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self { db }
    }

    fn search_table(
        &self,
        conn: &Connection,
        fts_table: &str,
        query: &str,
        source: &str,
        limit: usize,
    ) -> Result<Vec<Value>, String> {
        // For multi-column FTS tables (events: title, description), specify column 0.
        // For single-column tables, column 0 is the only column.
        let sql = format!(
            "SELECT {} AS source, rank, snippet({}, 0, '<b>', '</b>', '...', 64) AS snippet \
             FROM {} WHERE {} MATCH ?1 ORDER BY rank LIMIT ?2",
            quote(source),
            fts_table,
            fts_table,
            fts_table,
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params![query, limit as i64], |row| {
                Ok(serde_json::json!({
                    "source": row.get::<_, String>(0)?,
                    "snippet": row.get::<_, String>(2)?,
                }))
            })
            .map_err(|e| e.to_string())?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| e.to_string())?);
        }
        Ok(results)
    }

    pub async fn search(
        &self,
        query: &str,
        dimensions: Option<&str>,
        limit: usize,
    ) -> Result<ToolResult, ToolError> {
        let conn = self
            .db
            .lock()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        let tables: Vec<(&str, &str)> = match dimensions {
            Some("messages") => vec![("messages_fts", "message")],
            Some("memories") => vec![("memories_fts", "memory")],
            Some("notes") => vec![("notes_fts", "note")],
            Some("events") => vec![("events_fts", "event")],
            Some("tasks") => vec![("tasks_fts", "task")],
            Some("contacts") => vec![("contacts_fts", "contact")],
            _ => vec![
                ("messages_fts", "message"),
                ("memories_fts", "memory"),
                ("notes_fts", "note"),
                ("events_fts", "event"),
                ("tasks_fts", "task"),
                ("contacts_fts", "contact"),
            ],
        };

        let mut all_results = Vec::new();
        for (table, source) in tables {
            if let Ok(mut results) = self.search_table(&conn, table, query, source, limit) {
                all_results.append(&mut results);
            }
        }

        // Sort by rank (lower is better) and take top N
        all_results.sort_by(|a, b| {
            let ra = a["rank"].as_f64().unwrap_or(0.0);
            let rb = b["rank"].as_f64().unwrap_or(0.0);
            ra.partial_cmp(&rb).unwrap_or(std::cmp::Ordering::Equal)
        });
        all_results.truncate(limit);

        Ok(ToolResult {
            success: true,
            data: serde_json::to_value(&all_results).unwrap_or_default(),
            message: Some(format!("Found {} results", all_results.len())),
        })
    }
}

fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

#[async_trait]
impl Tool for UnifiedSearchTool {
    fn name(&self) -> &'static str {
        "unified_search"
    }

    fn description(&self) -> &'static str {
        "Buscar en todas las dimensiones (mensajes, memorias, notas, eventos, tareas, contactos)"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Texto a buscar" },
                "dimensions": {
                    "type": "string",
                    "enum": ["messages", "memories", "notes", "events", "tasks", "contacts"],
                    "description": "Limitar a una dimensión específica"
                },
                "limit": { "type": "integer", "description": "Máximo de resultados (default: 10)" }
            },
            "required": ["query"]
        })
    }

    fn permission(&self) -> Permission {
        Permission::NoConfirm
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, ToolError> {
        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
        if query.is_empty() {
            return Err(ToolError::InvalidArguments("query is required".into()));
        }
        let dimensions = args.get("dimensions").and_then(|v| v.as_str());
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
        self.search(query, dimensions, limit).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::run_migrations;

    fn setup() -> UnifiedSearchTool {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        // Create FTS triggers
        crate::db::fts::create_fts_triggers(&conn).unwrap();
        // Seed a profile
        conn.execute(
            "INSERT INTO profiles (id, name, preferences) VALUES ('p1', 'Test', '{}')",
            [],
        )
        .unwrap();
        // Seed a conversation
        conn.execute(
            "INSERT INTO conversations (id, title) VALUES ('c1', 'Test')",
            [],
        )
        .unwrap();
        // Insert a message (FTS trigger will index it)
        conn.execute(
            "INSERT INTO messages (id, conversation_id, role, content) \
             VALUES ('m1', 'c1', 'user', 'prueba de búsqueda unificada')",
            [],
        )
        .unwrap();
        // Insert a note
        conn.execute(
            "INSERT INTO notes (id, profile_id, content, category) \
             VALUES ('n1', 'p1', 'nota de prueba para búsqueda', 'idea')",
            [],
        )
        .unwrap();
        // Insert an event
        conn.execute(
            "INSERT INTO events (id, profile_id, title, description, start_time, end_time) \
             VALUES ('e1', 'p1', 'Evento de prueba', 'descripción del evento', \
             '2025-01-01T10:00:00Z', '2025-01-01T11:00:00Z')",
            [],
        )
        .unwrap();
        // Insert a task
        conn.execute(
            "INSERT INTO tasks (id, profile_id, content) \
             VALUES ('t1', 'p1', 'tarea de prueba para buscar')",
            [],
        )
        .unwrap();
        // Insert a contact
        conn.execute(
            "INSERT INTO contacts (id, profile_id, name) \
             VALUES ('c1', 'p1', 'Contacto de Prueba')",
            [],
        )
        .unwrap();

        let db = Arc::new(Mutex::new(conn));
        UnifiedSearchTool::new(db)
    }

    #[tokio::test]
    async fn test_search_rejects_empty_query() {
        let tool = setup();
        let result = tool.execute(serde_json::json!({"query": ""})).await;
        assert!(matches!(result, Err(ToolError::InvalidArguments(_))));
    }

    #[tokio::test]
    async fn test_search_finds_results_across_dimensions() {
        let tool = setup();
        let result = tool
            .execute(serde_json::json!({"query": "prueba", "limit": 10}))
            .await
            .unwrap();
        assert!(result.success);
        let results = result.data.as_array().unwrap();
        assert!(!results.is_empty(), "Should find at least one result");
    }

    #[tokio::test]
    async fn test_search_with_messages_dimension() {
        let tool = setup();
        let result = tool
            .execute(serde_json::json!({"query": "búsqueda", "dimensions": "messages", "limit": 5}))
            .await
            .unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_search_with_notes_dimension() {
        let tool = setup();
        let result = tool
            .execute(serde_json::json!({"query": "nota", "dimensions": "notes", "limit": 5}))
            .await
            .unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_search_respects_limit() {
        let tool = setup();
        let result = tool
            .execute(serde_json::json!({"query": "prueba", "limit": 1}))
            .await
            .unwrap();
        assert!(result.success);
        let results = result.data.as_array().unwrap();
        assert!(results.len() <= 1, "Should respect limit of 1");
    }

    #[tokio::test]
    async fn test_search_no_match_returns_empty() {
        let tool = setup();
        let result = tool
            .execute(serde_json::json!({"query": "zzzznoexiste", "limit": 10}))
            .await
            .unwrap();
        assert!(result.success);
        let results = result.data.as_array().unwrap();
        assert!(
            results.is_empty(),
            "Should return empty for non-matching query"
        );
    }
}
