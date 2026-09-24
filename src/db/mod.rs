pub mod fts;
pub mod repos;
pub mod schema;
pub mod vector;

use repos::tools::ToolsRepo;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

/// Type alias for the shared database pool used by all handlers.
pub type DbPool = Arc<Mutex<Connection>>;

/// Initialize a SQLite database at the given path.
///
/// 1. Opens (or creates) the database file.
/// 2. Configures WAL journal mode and foreign-key enforcement.
/// 3. Attempts to enable extension loading (needed for sqlite-vec in Phase 4).
/// 4. Runs all schema migrations.
/// 5. Seeds default tools if the tools table is empty.
/// 6. Registers the sqlite-vec extension (if available).
/// 7. Creates FTS5 triggers for full-text search.
pub fn init_db(db_path: &str) -> Result<Connection, Box<dyn std::error::Error>> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA foreign_keys=ON;",
    )?;

    // Enable extension loading for sqlite-vec (will be loaded in Phase 4).
    // SAFETY: load_extension_enable is unsafe because it allows loading
    // arbitrary shared libraries. We only call it for the bundled sqlite-vec
    // extension which is loaded in Phase 4. We discard the error because
    // the extension is not yet present.
    let _ = unsafe { conn.load_extension_enable() };

    schema::run_migrations(&conn)?;
    ToolsRepo::seed_defaults(&conn)?;

    // Initialize vector search (sqlite-vec registration, graceful fallback)
    let _ = vector::register_vector_ext(&conn);

    // Initialize FTS5 triggers
    fts::create_fts_triggers(&conn)?;

    Ok(conn)
}
