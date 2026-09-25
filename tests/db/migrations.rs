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
    alfred::db::schema::run_migrations(&pool).await.unwrap();
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
    alfred::db::schema::run_migrations(&pool).await.unwrap();

    // Second call — should not error
    alfred::db::schema::run_migrations(&pool).await.unwrap();
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
    alfred::db::schema::run_migrations(&pool).await.unwrap();
    alfred::db::schema::run_migrations(&pool).await.unwrap();

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
