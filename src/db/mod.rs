pub mod fts;
pub mod repos;
pub mod schema;
pub mod vector;

use repos::tools::ToolsRepo;
use sqlx::SqlitePool;

/// Type alias for the shared database pool used by all handlers.
pub type DbPool = sqlx::SqlitePool;

/// Initialize a SQLite database at the given path.
///
/// 1. Opens (or creates) the database file via sqlx pool.
/// 2. Configures WAL journal mode and foreign-key enforcement.
/// 3. Runs all sqlx schema migrations.
/// 4. Seeds default tools if the tools table is empty.
/// 5. Creates FTS5 triggers for full-text search.
pub async fn init_db(db_path: &str) -> Result<DbPool, Box<dyn std::error::Error>> {
    use sqlx::sqlite::SqliteConnectOptions;
    use std::str::FromStr;

    let options = SqliteConnectOptions::from_str(db_path)?
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .foreign_keys(true)
        .create_if_missing(true);

    let pool = SqlitePool::connect_with(options).await?;

    // Run sqlx migrations
    sqlx::migrate::Migrator::new(std::path::Path::new("migrations"))
        .await?
        .run(&pool)
        .await?;

    // Seed default tools
    ToolsRepo::seed_defaults(&pool).await?;

    // Initialize FTS5 triggers
    fts::create_fts_triggers(&pool).await?;

    Ok(pool)
}
