use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;

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
    valet::db::schema::run_migrations(&pool).await.unwrap();
    pool
}

/// Asserts that `run_migrations` creates the `messages` table
/// and does NOT include a `conversation_id` column.
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

/// Asserts that `run_migrations` does NOT create the `conversations` table.
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

/// Asserts that migration is idempotent (can be called twice).
#[tokio::test]
async fn test_migration_is_idempotent() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(":memory:")
                .create_if_missing(true),
        )
        .await
        .unwrap();

    // First call
    valet::db::schema::run_migrations(&pool).await.unwrap();

    // Second call — should not error
    valet::db::schema::run_migrations(&pool).await.unwrap();
}

// ── F5c: Tools de Valor — Schema tests ─────────────────────────────────────

/// Asserts that `run_migrations` creates the `meal_plans` table.
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

/// Asserts that `run_migrations` creates the `shopping_list` table.
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

/// Asserts that `run_migrations` creates the `habits` table.
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

/// Asserts that `run_migrations` creates the `habit_logs` table.
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

/// Asserts idempotency covers the new F5c tables.
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

    // Call twice
    valet::db::schema::run_migrations(&pool).await.unwrap();
    valet::db::schema::run_migrations(&pool).await.unwrap();

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

// ── Prompts migration (20260929000001_prompts.sql) ─────────────────────────

/// Reads the prompts migration SQL from disk.
fn prompts_migration_sql() -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("migrations/20260929000001_prompts.sql");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", path.display()))
}

/// Reads a single setting value, panicking if the key is missing.
async fn setting_value(pool: &SqlitePool, key: &str) -> String {
    sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?1")
        .bind(key)
        .fetch_one(pool)
        .await
        .unwrap_or_else(|e| panic!("Failed to read setting '{key}': {e}"))
}

/// Asserts that `run_migrations` seeds a non-empty `system_prompt`.
#[tokio::test]
async fn test_migration_seeds_system_prompt() {
    let pool = setup().await;

    let value = setting_value(&pool, "system_prompt").await;
    assert!(!value.is_empty(), "system_prompt should not be empty");
    assert!(
        value.contains("asistente personal británico"),
        "system_prompt should contain the British assistant personality"
    );
}

/// Asserts that `run_migrations` seeds the archivist prompt with its placeholder.
#[tokio::test]
async fn test_migration_seeds_archivist_prompt() {
    let pool = setup().await;

    let value = setting_value(&pool, "archivist_prompt").await;
    assert!(!value.is_empty(), "archivist_prompt should not be empty");
    assert!(
        value.contains("archivista de memoria"),
        "archivist_prompt should contain 'archivista de memoria'"
    );
    assert!(
        value.contains("{{ BLOQUE_DE_MENSAJES }}"),
        "archivist_prompt should contain the message block placeholder"
    );
}

/// Asserts that `run_migrations` seeds the collapse prompt.
#[tokio::test]
async fn test_migration_seeds_collapse_prompt() {
    let pool = setup().await;

    let value = setting_value(&pool, "collapse_prompt").await;
    assert!(!value.is_empty(), "collapse_prompt should not be empty");
    assert!(
        value.contains("Resume el siguiente texto"),
        "collapse_prompt should contain 'Resume el siguiente texto'"
    );
}

/// An old database with an empty `system_prompt` gets backfilled by the migration.
#[tokio::test]
async fn test_migration_fills_empty_system_prompt() {
    let pool = setup().await;

    sqlx::query("UPDATE settings SET value = '' WHERE key = 'system_prompt'")
        .execute(&pool)
        .await
        .unwrap();

    let sql = prompts_migration_sql();
    sqlx::raw_sql(sqlx::AssertSqlSafe(sql.as_str()))
        .execute(&pool)
        .await
        .unwrap();

    let value = setting_value(&pool, "system_prompt").await;
    assert!(
        !value.is_empty(),
        "system_prompt should be backfilled when it was empty"
    );
}

/// A non-empty custom `system_prompt` is preserved by the migration.
#[tokio::test]
async fn test_migration_respects_custom_system_prompt() {
    let pool = setup().await;

    sqlx::query(
        "UPDATE settings SET value = 'Mi prompt personalizado' WHERE key = 'system_prompt'",
    )
    .execute(&pool)
    .await
    .unwrap();

    let sql = prompts_migration_sql();
    sqlx::raw_sql(sqlx::AssertSqlSafe(sql.as_str()))
        .execute(&pool)
        .await
        .unwrap();

    let value = setting_value(&pool, "system_prompt").await;
    assert_eq!(
        value, "Mi prompt personalizado",
        "A non-empty custom system_prompt must be preserved"
    );
}

/// Running the prompts migration twice is idempotent and keeps one row per key.
#[tokio::test]
async fn test_migration_prompts_idempotent() {
    let pool = setup().await;

    let sql = prompts_migration_sql();
    sqlx::raw_sql(sqlx::AssertSqlSafe(sql.as_str()))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::raw_sql(sqlx::AssertSqlSafe(sql.as_str()))
        .execute(&pool)
        .await
        .unwrap();

    for key in &["system_prompt", "archivist_prompt", "collapse_prompt"] {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM settings WHERE key = ?1")
            .bind(key)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1, "Expected exactly one row for key '{key}'");
    }
}
