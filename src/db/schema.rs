use sqlx::SqlitePool;

/// Run all database migrations using sqlx's embedded migration system.
///
/// Migrations live in the `migrations/` directory. This function resolves
/// the path relative to `CARGO_MANIFEST_DIR` (embedded at compile time) to
/// work reliably regardless of the process's current working directory.
pub async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let migrations_path = manifest.join("migrations");
    sqlx::migrate::Migrator::new(migrations_path)
        .await?
        .run(pool)
        .await?;
    Ok(())
}

/// Seed default settings into the database.
/// Called after migrations to ensure required settings exist.
pub async fn seed_default_settings(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    crate::db::repos::settings::SettingsRepo::seed_defaults(pool).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    async fn setup() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await
            .unwrap();
        run_migrations(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn test_migrations_does_not_create_conversations() {
        let pool = setup().await;

        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='conversations'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(
            count, 0,
            "Table 'conversations' should NOT exist after migration"
        );
    }

    #[tokio::test]
    async fn test_migrations_creates_messages_table() {
        let pool = setup().await;

        let has_table: bool = sqlx::query_scalar(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='messages'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert!(
            has_table,
            "Expected 'messages' table to exist after migration"
        );

        // Verify no conversation_id column exists
        let column_names: Vec<String> =
            sqlx::query_scalar("SELECT name FROM pragma_table_info('messages')")
                .fetch_all(&pool)
                .await
                .unwrap();

        assert!(
            !column_names.contains(&"conversation_id".to_string()),
            "Column 'conversation_id' should NOT exist in messages table"
        );
    }

    #[tokio::test]
    async fn test_migrations_creates_meal_plans_table() {
        let pool = setup().await;

        let has_table: bool = sqlx::query_scalar(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='meal_plans'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert!(
            has_table,
            "Expected 'meal_plans' table to exist after migration"
        );
    }

    #[tokio::test]
    async fn test_migrations_creates_shopping_list_table() {
        let pool = setup().await;

        let has_table: bool = sqlx::query_scalar(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='shopping_list'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert!(
            has_table,
            "Expected 'shopping_list' table to exist after migration"
        );
    }

    #[tokio::test]
    async fn test_migrations_creates_habits_table() {
        let pool = setup().await;

        let has_table: bool = sqlx::query_scalar(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='habits'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert!(
            has_table,
            "Expected 'habits' table to exist after migration"
        );
    }

    #[tokio::test]
    async fn test_migrations_creates_habit_logs_table() {
        let pool = setup().await;

        let has_table: bool = sqlx::query_scalar(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='habit_logs'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert!(
            has_table,
            "Expected 'habit_logs' table to exist after migration"
        );
    }

    #[tokio::test]
    async fn test_idempotent_includes_new_tables() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await
            .unwrap();

        run_migrations(&pool).await.unwrap();
        run_migrations(&pool).await.unwrap();

        let tables: Vec<String> =
            sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
                .fetch_all(&pool)
                .await
                .unwrap();

        for table in &["meal_plans", "shopping_list", "habits", "habit_logs"] {
            assert!(
                tables.contains(&table.to_string()),
                "Expected '{table}' table after idempotent migration"
            );
        }
    }

    #[tokio::test]
    async fn test_messages_table_has_new_columns() {
        let pool = setup().await;

        let column_names: Vec<String> =
            sqlx::query_scalar("SELECT name FROM pragma_table_info('messages')")
                .fetch_all(&pool)
                .await
                .unwrap();

        assert!(
            column_names.contains(&"tokens_count".to_string()),
            "Column 'tokens_count' should exist in messages table"
        );
        assert!(
            column_names.contains(&"collapsed_content".to_string()),
            "Column 'collapsed_content' should exist in messages table"
        );
        assert!(
            column_names.contains(&"collapsed_tokens_count".to_string()),
            "Column 'collapsed_tokens_count' should exist in messages table"
        );
        assert!(
            column_names.contains(&"is_indexed".to_string()),
            "Column 'is_indexed' should exist in messages table"
        );
        assert!(
            column_names.contains(&"summary_ref".to_string()),
            "Column 'summary_ref' should exist in messages table"
        );
    }
}
