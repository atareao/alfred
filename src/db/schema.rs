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
    async fn test_messages_table_has_location_column() {
        let pool = setup().await;

        let column_names: Vec<String> =
            sqlx::query_scalar("SELECT name FROM pragma_table_info('messages')")
                .fetch_all(&pool)
                .await
                .unwrap();

        assert!(
            column_names.contains(&"location".to_string()),
            "Column 'location' should exist in messages table"
        );
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

    // ─── Episodic memory migration tests ───────────────────────────────────────

    #[tokio::test]
    async fn test_memory_table_exists_with_columns() {
        let pool = setup().await;

        let has_table: bool = sqlx::query_scalar(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='memory'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert!(
            has_table,
            "Expected 'memory' table to exist after migration"
        );

        let columns: Vec<(i64, String, String, i64, Option<String>, i64)> = sqlx::query_as(
            "SELECT cid, name, type, \"notnull\", dflt_value, pk FROM pragma_table_info('memory')",
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        let col_map: std::collections::BTreeMap<String, (String, Option<String>, i64)> = columns
            .into_iter()
            .map(|(_cid, name, ty, _notnull, dflt, pk)| (name, (ty, dflt, pk)))
            .collect();

        // id TEXT PRIMARY KEY
        let (ty, dflt, pk) = col_map.get("id").expect("Column 'id' should exist");
        assert_eq!(ty.to_uppercase(), "TEXT", "id should be TEXT");
        assert_eq!(*pk, 1, "id should be PRIMARY KEY");

        // content TEXT NOT NULL
        let (ty, dflt, pk) = col_map
            .get("content")
            .expect("Column 'content' should exist");
        assert_eq!(ty.to_uppercase(), "TEXT", "content should be TEXT");
        assert_eq!(*pk, 0, "content should not be PK");

        // tokens_count INTEGER NOT NULL DEFAULT 0
        let (ty, dflt, pk) = col_map
            .get("tokens_count")
            .expect("Column 'tokens_count' should exist");
        assert_eq!(
            ty.to_uppercase(),
            "INTEGER",
            "tokens_count should be INTEGER"
        );
        assert_eq!(
            dflt.as_deref(),
            Some("0"),
            "tokens_count should default to 0"
        );
        assert_eq!(*pk, 0, "tokens_count should not be PK");

        // created_at TEXT
        let (ty, dflt, _pk) = col_map
            .get("created_at")
            .expect("Column 'created_at' should exist");
        assert_eq!(ty.to_uppercase(), "TEXT", "created_at should be TEXT");

        // metadata TEXT DEFAULT '{}'
        let (ty, dflt, _pk) = col_map
            .get("metadata")
            .expect("Column 'metadata' should exist");
        assert_eq!(ty.to_uppercase(), "TEXT", "metadata should be TEXT");
        assert_eq!(
            dflt.as_deref(),
            Some("'{}'"),
            "metadata should default to '{{}}'"
        );
    }

    #[tokio::test]
    async fn test_vec_memory_virtual_table_exists() {
        let pool = setup().await;

        let has_table: bool = sqlx::query_scalar(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='vec_memory'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert!(
            has_table,
            "Expected 'vec_memory' virtual table to exist after migration"
        );

        let ddl: Option<String> = sqlx::query_scalar(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='vec_memory'",
        )
        .fetch_optional(&pool)
        .await
        .unwrap();

        assert!(
            ddl.as_ref().is_some_and(|s| {
                let upper = s.trim().to_uppercase();
                upper.starts_with("CREATE TABLE") || upper.starts_with("CREATE VIRTUAL TABLE")
            }),
            "vec_memory must be a TABLE (regular or virtual)"
        );
    }

    #[tokio::test]
    async fn test_idx_messages_unindexed_exists() {
        let pool = setup().await;

        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_messages_unindexed'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(
            count, 1,
            "Index 'idx_messages_unindexed' should exist on messages(created_at) WHERE is_indexed = 0"
        );
    }

    #[tokio::test]
    async fn test_idx_messages_summary_ref_exists() {
        let pool = setup().await;

        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_messages_summary_ref'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(
            count, 1,
            "Index 'idx_messages_summary_ref' should exist on messages(summary_ref) WHERE summary_ref IS NOT NULL"
        );
    }

    #[tokio::test]
    async fn test_legacy_tables_do_not_exist() {
        let pool = setup().await;

        let tables: Vec<String> =
            sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
                .fetch_all(&pool)
                .await
                .unwrap();

        for legacy in &["memories", "memory_embeddings", "memories_fts"] {
            assert!(
                !tables.contains(&legacy.to_string()),
                "Legacy table '{legacy}' should NOT exist after migration"
            );
        }
    }

    #[tokio::test]
    async fn test_memory_migration_is_idempotent() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await
            .unwrap();

        // Run twice – second run must not error
        run_migrations(&pool).await.unwrap();
        run_migrations(&pool).await.unwrap();

        // Verify memory table still looks correct after second run
        let has_memory: bool = sqlx::query_scalar(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='memory'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert!(
            has_memory,
            "memory table should survive idempotent migration"
        );

        let has_vec: bool = sqlx::query_scalar(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='vec_memory'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert!(
            has_vec,
            "vec_memory table should survive idempotent migration"
        );
    }
}
