use sqlx::Row;
use sqlx::SqlitePool;
use std::collections::HashMap;

pub struct SettingsRepo;

impl SettingsRepo {
    /// Get a single setting by key. Returns None if not found.
    pub async fn get(pool: &SqlitePool, key: &str) -> Result<Option<String>, sqlx::Error> {
        let row = sqlx::query("SELECT value FROM settings WHERE key = ?1")
            .bind(key)
            .fetch_optional(pool)
            .await?;
        Ok(row.map(|r| r.get(0)))
    }

    /// Set a setting (insert or update). Sets updated_at to current timestamp.
    pub async fn set(pool: &SqlitePool, key: &str, value: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')",
        )
        .bind(key)
        .bind(value)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Get all settings as a HashMap.
    pub async fn get_all(pool: &SqlitePool) -> Result<HashMap<String, String>, sqlx::Error> {
        let rows = sqlx::query("SELECT key, value FROM settings")
            .fetch_all(pool)
            .await?;
        let mut map = HashMap::new();
        for row in rows {
            let k: String = row.get(0);
            let v: String = row.get(1);
            map.insert(k, v);
        }
        Ok(map)
    }

    /// Delete a setting by key.
    pub async fn delete(pool: &SqlitePool, key: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM settings WHERE key = ?1")
            .bind(key)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Seed default settings values.
    pub async fn seed_defaults(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)")
            .bind("max_window_tokens")
            .bind("10000")
            .execute(pool)
            .await?;
        sqlx::query("INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)")
            .bind("system_prompt")
            .bind("")
            .execute(pool)
            .await?;
        sqlx::query("INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)")
            .bind("collapse_prompt")
            .bind("Resume el siguiente texto manteniendo la información clave, los datos importantes y el contexto necesario. Sé conciso.")
            .execute(pool)
            .await?;
        sqlx::query("INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)")
            .bind("message_page_size")
            .bind("50")
            .execute(pool)
            .await?;
        // API keys for external services (set via UI, fallback to ENV)
        sqlx::query("INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)")
            .bind("google_places_api_key")
            .bind("")
            .execute(pool)
            .await?;
        sqlx::query("INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)")
            .bind("brave_search_api_key")
            .bind("")
            .execute(pool)
            .await?;
        sqlx::query("INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)")
            .bind("openweather_api_key")
            .bind("")
            .execute(pool)
            .await?;
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
        SettingsRepo::seed_defaults(&pool).await?;
        Ok(pool)
    }

    #[tokio::test]
    async fn test_get_default_value() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        let value = SettingsRepo::get(&pool, "max_window_tokens").await.unwrap();
        assert_eq!(value, Some("10000".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_get_nonexistent() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        let value = SettingsRepo::get(&pool, "nonexistent").await.unwrap();
        assert_eq!(value, None);

        Ok(())
    }

    #[tokio::test]
    async fn test_set_inserts_new() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        SettingsRepo::set(&pool, "foo", "bar").await.unwrap();
        let value = SettingsRepo::get(&pool, "foo").await.unwrap();
        assert_eq!(value, Some("bar".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_set_updates_existing() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        SettingsRepo::set(&pool, "foo", "bar").await.unwrap();
        SettingsRepo::set(&pool, "foo", "baz").await.unwrap();
        let value = SettingsRepo::get(&pool, "foo").await.unwrap();
        assert_eq!(value, Some("baz".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_get_all() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        let all = SettingsRepo::get_all(&pool).await.unwrap();
        assert!(all.contains_key("max_window_tokens"));
        assert!(all.contains_key("system_prompt"));
        assert!(all.contains_key("collapse_prompt"));

        Ok(())
    }

    #[tokio::test]
    async fn test_collapse_prompt_seeded() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        let value = SettingsRepo::get(&pool, "collapse_prompt").await.unwrap();
        assert!(
            value.is_some(),
            "collapse_prompt should be seeded after migrations"
        );
        let prompt = value.unwrap();
        assert!(
            !prompt.is_empty(),
            "collapse_prompt should have a non-empty default value"
        );
        assert!(
            prompt.contains("Resume"),
            "Default collapse_prompt should be in Spanish, containing 'Resume'"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_delete() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        SettingsRepo::set(&pool, "foo", "bar").await.unwrap();
        SettingsRepo::delete(&pool, "foo").await.unwrap();
        let value = SettingsRepo::get(&pool, "foo").await.unwrap();
        assert_eq!(value, None);

        Ok(())
    }

    #[tokio::test]
    async fn test_message_page_size_seeded() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        let value = SettingsRepo::get(&pool, "message_page_size").await.unwrap();
        assert_eq!(value, Some("50".to_string()));

        Ok(())
    }
}
