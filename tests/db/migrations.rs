use rusqlite::Connection;

/// RED phase test: asserts that `run_migrations` creates the `conversations` table.
/// This will FAIL because the stub does not run any DDL.
#[test]
fn test_migrations_creates_conversations_table() {
    let conn = Connection::open_in_memory().unwrap();
    alfred::db::schema::run_migrations(&conn).unwrap();

    let has_table: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='conversations'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(
        has_table,
        "Expected 'conversations' table to exist after migration"
    );
}

/// RED phase test: asserts that `run_migrations` creates the `messages` table.
/// This will FAIL because the stub does not run any DDL.
#[test]
fn test_migrations_creates_messages_table() {
    let conn = Connection::open_in_memory().unwrap();
    alfred::db::schema::run_migrations(&conn).unwrap();

    let has_table: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='messages'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(
        has_table,
        "Expected 'messages' table to exist after migration"
    );
}

/// RED phase test: asserts that migration is idempotent (can be called twice).
/// This will FAIL because the first assertion already fails.
#[test]
fn test_migration_is_idempotent() {
    let conn = Connection::open_in_memory().unwrap();

    // First call
    alfred::db::schema::run_migrations(&conn).unwrap();

    // Second call — should not error
    alfred::db::schema::run_migrations(&conn).unwrap();

    // Tables should exist
    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert!(
        tables.contains(&"conversations".to_string()),
        "Expected 'conversations' table after idempotent migration"
    );
}

// ── F5c: Tools de Valor — Schema tests ─────────────────────────────────────

/// RED phase test: asserts that `run_migrations` creates the `meal_plans` table.
#[test]
fn test_migrations_creates_meal_plans_table() {
    let conn = Connection::open_in_memory().unwrap();
    alfred::db::schema::run_migrations(&conn).unwrap();

    let has_table: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='meal_plans'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(
        has_table,
        "Expected 'meal_plans' table to exist after migration"
    );
}

/// RED phase test: asserts that `run_migrations` creates the `shopping_list` table.
#[test]
fn test_migrations_creates_shopping_list_table() {
    let conn = Connection::open_in_memory().unwrap();
    alfred::db::schema::run_migrations(&conn).unwrap();

    let has_table: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='shopping_list'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(
        has_table,
        "Expected 'shopping_list' table to exist after migration"
    );
}

/// RED phase test: asserts that `run_migrations` creates the `habits` table.
#[test]
fn test_migrations_creates_habits_table() {
    let conn = Connection::open_in_memory().unwrap();
    alfred::db::schema::run_migrations(&conn).unwrap();

    let has_table: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='habits'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(
        has_table,
        "Expected 'habits' table to exist after migration"
    );
}

/// RED phase test: asserts that `run_migrations` creates the `habit_logs` table.
#[test]
fn test_migrations_creates_habit_logs_table() {
    let conn = Connection::open_in_memory().unwrap();
    alfred::db::schema::run_migrations(&conn).unwrap();

    let has_table: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='habit_logs'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(
        has_table,
        "Expected 'habit_logs' table to exist after migration"
    );
}

/// RED phase test: asserts idempotency covers the new F5c tables.
#[test]
fn test_idempotent_includes_new_tables() {
    let conn = Connection::open_in_memory().unwrap();

    // Call twice
    alfred::db::schema::run_migrations(&conn).unwrap();
    alfred::db::schema::run_migrations(&conn).unwrap();

    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    for table in &["meal_plans", "shopping_list", "habits", "habit_logs"] {
        assert!(
            tables.contains(&table.to_string()),
            "Expected '{table}' table after idempotent migration"
        );
    }
}
