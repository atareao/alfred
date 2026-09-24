use std::process::Command;

/// RED phase test: verifies that the seed binary creates profiles.
#[test]
fn test_seed_binary_populates_database() {
    // Given a fresh database file
    let tmp_dir = std::env::temp_dir();
    let db_path = tmp_dir.join(format!("test_seed_{}.db", std::process::id()));
    let db_path_str = db_path.to_string_lossy().to_string();

    // Clean up any previous test run
    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_file(db_path.with_extension("db-wal"));
    let _ = std::fs::remove_file(db_path.with_extension("db-shm"));

    // When the seed binary is executed
    let status = Command::new(env!("CARGO_BIN_EXE_seed"))
        .env("DATABASE_URL", &db_path_str)
        .status()
        .expect("Failed to run seed binary");

    // Then it should exit successfully
    assert!(status.success(), "Seed binary should exit with status 0");

    // And the database should have profiles
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM profiles", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 2, "Expected 2 profiles to be seeded");

    // And the database should have conversations
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM conversations", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1, "Expected 1 conversation to be seeded");

    // Clean up
    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_file(db_path.with_extension("db-wal"));
    let _ = std::fs::remove_file(db_path.with_extension("db-shm"));
}
