use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use std::process::Command;

/// Verifies that the seed binary creates profiles and other seed data in the
/// database, then reads the resulting database using sqlx.
#[tokio::test]
async fn test_seed_binary_populates_database() {
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

    // And the database should have profiles — read using sqlx
    let pool = SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(false)
            .read_only(true),
    )
    .await
    .expect("Failed to open seeded database with sqlx");

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM profiles")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 2, "Expected 2 profiles to be seeded");

    // Close pool before cleanup
    pool.close().await;

    // Clean up
    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_file(db_path.with_extension("db-wal"));
    let _ = std::fs::remove_file(db_path.with_extension("db-shm"));
}
