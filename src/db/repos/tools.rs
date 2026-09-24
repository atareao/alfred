use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::models::Tool;

pub struct ToolsRepo;

impl ToolsRepo {
    pub fn list(conn: &Connection) -> Result<Vec<Tool>, rusqlite::Error> {
        let mut stmt =
            conn.prepare("SELECT id, name, description, enabled FROM tools ORDER BY name")?;
        let mut rows = stmt.query([])?;
        let mut items = Vec::new();
        while let Some(row) = rows.next()? {
            let enabled_int: i32 = row.get(3)?;
            items.push(Tool {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                enabled: enabled_int != 0,
            });
        }
        Ok(items)
    }

    pub fn find_by_name(conn: &Connection, name: &str) -> Result<Option<Tool>, rusqlite::Error> {
        let mut stmt =
            conn.prepare("SELECT id, name, description, enabled FROM tools WHERE name = ?1")?;
        let mut rows = stmt.query(params![name])?;
        match rows.next()? {
            Some(row) => {
                let enabled_int: i32 = row.get(3)?;
                Ok(Some(Tool {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    enabled: enabled_int != 0,
                }))
            }
            None => Ok(None),
        }
    }

    pub fn toggle_enabled(conn: &Connection, id: &str) -> Result<Option<Tool>, rusqlite::Error> {
        conn.execute(
            "UPDATE tools SET enabled = CASE WHEN enabled = 1 THEN 0 ELSE 1 END WHERE id = ?1",
            params![id],
        )?;

        let mut stmt =
            conn.prepare("SELECT id, name, description, enabled FROM tools WHERE id = ?1")?;
        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => {
                let enabled_int: i32 = row.get(3)?;
                Ok(Some(Tool {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    enabled: enabled_int != 0,
                }))
            }
            None => Ok(None),
        }
    }

    pub fn seed_defaults(conn: &Connection) -> Result<(), rusqlite::Error> {
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM tools", [], |row| row.get(0))?;
        if count > 0 {
            return Ok(());
        }

        let defaults = vec![
            ("calendar", "Gestión de agenda y eventos"),
            ("tasks", "Gestión de tareas pendientes"),
            ("weather", "Consulta del clima"),
            ("geo", "Geolocalización y búsqueda de lugares"),
            ("meals", "Planificación de comidas y lista de la compra"),
            ("habits", "Seguimiento de hábitos"),
            ("knowledge", "Notas y conocimiento personal"),
            ("contacts", "Gestión de contactos"),
            ("reminders", "Recordatorios con notificaciones"),
            (
                "unified_search",
                "Búsqueda unificada en todas las dimensiones (mensajes, memorias, notas, eventos, tareas, contactos)",
            ),
        ];

        for (name, description) in defaults {
            let id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO tools (id, name, description, enabled) VALUES (?1, ?2, ?3, 1)",
                params![id, name, description],
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::run_migrations;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        ToolsRepo::seed_defaults(&conn).unwrap();
        conn
    }

    #[test]
    fn test_list_tools() {
        let conn = setup();
        let tools = ToolsRepo::list(&conn).unwrap();
        assert_eq!(tools.len(), 10);
        assert!(tools.iter().any(|t| t.name == "weather"));
        assert!(tools.iter().any(|t| t.name == "unified_search"));
    }

    #[test]
    fn test_toggle_enabled() {
        let conn = setup();
        let tools = ToolsRepo::list(&conn).unwrap();
        let tool = tools.into_iter().find(|t| t.name == "weather").unwrap();
        assert!(tool.enabled);

        let toggled = ToolsRepo::toggle_enabled(&conn, &tool.id).unwrap().unwrap();
        assert!(!toggled.enabled);
    }

    #[test]
    fn test_toggle_not_found() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let result = ToolsRepo::toggle_enabled(&conn, "nonexistent").unwrap();
        assert!(result.is_none());
    }
}
