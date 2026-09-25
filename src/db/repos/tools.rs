use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::models::Tool;

pub struct ToolsRepo;

impl ToolsRepo {
    pub async fn list(pool: &SqlitePool) -> Result<Vec<Tool>, sqlx::Error> {
        let rows = sqlx::query("SELECT id, name, description, enabled FROM tools ORDER BY name")
            .fetch_all(pool)
            .await?;

        let items: Vec<Tool> = rows
            .iter()
            .map(|row| Tool {
                id: row.get(0),
                name: row.get(1),
                description: row.get(2),
                enabled: row.get::<bool, _>(3),
            })
            .collect();

        Ok(items)
    }

    pub async fn find_by_name(pool: &SqlitePool, name: &str) -> Result<Option<Tool>, sqlx::Error> {
        let row = sqlx::query("SELECT id, name, description, enabled FROM tools WHERE name = ?1")
            .bind(name)
            .fetch_optional(pool)
            .await?;

        Ok(row.map(|r| Tool {
            id: r.get(0),
            name: r.get(1),
            description: r.get(2),
            enabled: r.get::<bool, _>(3),
        }))
    }

    pub async fn toggle_enabled(pool: &SqlitePool, id: &str) -> Result<Option<Tool>, sqlx::Error> {
        sqlx::query(
            "UPDATE tools SET enabled = CASE WHEN enabled = 1 THEN 0 ELSE 1 END WHERE id = ?1",
        )
        .bind(id)
        .execute(pool)
        .await?;

        let row = sqlx::query("SELECT id, name, description, enabled FROM tools WHERE id = ?1")
            .bind(id)
            .fetch_optional(pool)
            .await?;

        Ok(row.map(|r| Tool {
            id: r.get(0),
            name: r.get(1),
            description: r.get(2),
            enabled: r.get::<bool, _>(3),
        }))
    }

    pub async fn seed_defaults(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tools")
            .fetch_one(pool)
            .await?;

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
            sqlx::query(
                "INSERT INTO tools (id, name, description, enabled) VALUES (?1, ?2, ?3, 1)",
            )
            .bind(id)
            .bind(name)
            .bind(description)
            .execute(pool)
            .await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    async fn setup() -> Result<SqlitePool, sqlx::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await?;
        sqlx::migrate::Migrator::new(std::path::Path::new("migrations"))
            .await
            .unwrap()
            .run(&pool)
            .await
            .unwrap();
        ToolsRepo::seed_defaults(&pool).await?;
        Ok(pool)
    }

    #[tokio::test]
    async fn test_list_tools() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        let tools = ToolsRepo::list(&pool).await?;
        assert_eq!(tools.len(), 10);
        assert!(tools.iter().any(|t| t.name == "weather"));
        assert!(tools.iter().any(|t| t.name == "unified_search"));
        Ok(())
    }

    #[tokio::test]
    async fn test_toggle_enabled() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        let tools = ToolsRepo::list(&pool).await?;
        let tool = tools.into_iter().find(|t| t.name == "weather").unwrap();
        assert!(tool.enabled);

        let toggled = ToolsRepo::toggle_enabled(&pool, &tool.id).await?.unwrap();
        assert!(!toggled.enabled);
        Ok(())
    }

    #[tokio::test]
    async fn test_toggle_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await?;
        sqlx::migrate::Migrator::new(std::path::Path::new("migrations"))
            .await
            .unwrap()
            .run(&pool)
            .await
            .unwrap();
        let result = ToolsRepo::toggle_enabled(&pool, "nonexistent").await?;
        assert!(result.is_none());
        Ok(())
    }
}
