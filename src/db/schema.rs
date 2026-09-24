use rusqlite::Connection;

/// Run all database migrations.
///
/// Creates all core tables (`conversations`, `messages`, `profiles`, `memories`,
/// `tools`, `message_embeddings`, `memory_embeddings`) unconditionally.
/// FTS5 virtual tables are wrapped in try-blocks for resilience; they should
/// succeed since FTS5 is built into bundled SQLite.
/// sqlite-vec will be wired in a future phase.
pub fn run_migrations(conn: &Connection) -> Result<(), rusqlite::Error> {
    // ── Core tables ──────────────────────────────────────────────────────────
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS conversations (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL REFERENCES conversations(id),
            role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system', 'tool')),
            content TEXT NOT NULL,
            tool_calls TEXT,
            tool_results TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS profiles (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            avatar_url TEXT,
            preferences TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS memories (
            id TEXT PRIMARY KEY,
            profile_id TEXT NOT NULL REFERENCES profiles(id),
            content TEXT NOT NULL,
            category TEXT NOT NULL DEFAULT 'general',
            source TEXT NOT NULL DEFAULT 'manual',
            embedding_id TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS tools (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            description TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1
        );
        ",
    )?;

    // Migrate: add new columns to messages table (safe repeated runs)
    let has_tokens_count: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('messages') WHERE name='tokens_count'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;
    if !has_tokens_count {
        conn.execute_batch(
            "ALTER TABLE messages ADD COLUMN tokens_count INTEGER NOT NULL DEFAULT 0;
             ALTER TABLE messages ADD COLUMN collapsed_content TEXT;
             ALTER TABLE messages ADD COLUMN collapsed_tokens_count INTEGER NOT NULL DEFAULT 0;
             ALTER TABLE messages ADD COLUMN is_indexed INTEGER NOT NULL DEFAULT 0;
             ALTER TABLE messages ADD COLUMN summary_ref TEXT;",
        )?;
    }

    // ── Embedding tables (regular tables; vec0 will be added in a future phase) ─
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS message_embeddings (
            id TEXT PRIMARY KEY,
            embedding TEXT NOT NULL DEFAULT '[]'
        );

        CREATE TABLE IF NOT EXISTS memory_embeddings (
            id TEXT PRIMARY KEY,
            embedding TEXT NOT NULL DEFAULT '[]'
        );",
    )?;

    // ── FTS5 virtual tables ───────────────────────────────────────────────────
    let _ = conn.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
            content,
            content=messages,
            content_rowid=rowid
        );",
    );

    let _ = conn.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
            content,
            content=memories,
            content_rowid=rowid
        );",
    );

    let _ = conn.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(
            content,
            content=notes,
            content_rowid=rowid
        );",
    );

    let _ = conn.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS events_fts USING fts5(
            title, description,
            content=events,
            content_rowid=rowid
        );",
    );

    let _ = conn.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS tasks_fts USING fts5(
            content,
            content=tasks,
            content_rowid=rowid
        );",
    );

    let _ = conn.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS contacts_fts USING fts5(
            name,
            content=contacts,
            content_rowid=rowid
        );",
    );

    // ── Tools Core tables (events, tasks, reminders, notes, contacts) ──────
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS events (
            id TEXT PRIMARY KEY,
            profile_id TEXT NOT NULL REFERENCES profiles(id),
            title TEXT NOT NULL,
            description TEXT,
            start_time TEXT NOT NULL,
            end_time TEXT NOT NULL,
            location TEXT,
            scope TEXT NOT NULL DEFAULT 'shared' CHECK(scope IN ('shared', 'personal')),
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            profile_id TEXT NOT NULL REFERENCES profiles(id),
            content TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'completed', 'cancelled')),
            priority TEXT NOT NULL DEFAULT 'medium' CHECK(priority IN ('low', 'medium', 'high')),
            project TEXT,
            due_date TEXT,
            scope TEXT NOT NULL DEFAULT 'shared' CHECK(scope IN ('shared', 'personal')),
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS reminders (
            id TEXT PRIMARY KEY,
            profile_id TEXT NOT NULL REFERENCES profiles(id),
            text TEXT NOT NULL,
            datetime TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'dismissed', 'snoozed')),
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS notes (
            id TEXT PRIMARY KEY,
            profile_id TEXT NOT NULL REFERENCES profiles(id),
            content TEXT NOT NULL,
            category TEXT NOT NULL DEFAULT 'idea' CHECK(category IN ('idea', 'journal', 'fact', 'todo')),
            tags TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS contacts (
            id TEXT PRIMARY KEY,
            profile_id TEXT NOT NULL REFERENCES profiles(id),
            name TEXT NOT NULL,
            phone TEXT,
            email TEXT,
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        ",
    )?;

    // ── F5c: Tools de Valor tables (meal_plans, shopping_list, habits, habit_logs) ──
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS meal_plans (
            id TEXT PRIMARY KEY,
            profile_id TEXT NOT NULL REFERENCES profiles(id),
            week_start TEXT NOT NULL,
            meals TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_meal_plans_profile_week
            ON meal_plans(profile_id, week_start);

        CREATE TABLE IF NOT EXISTS shopping_list (
            id TEXT PRIMARY KEY,
            profile_id TEXT NOT NULL REFERENCES profiles(id),
            item TEXT NOT NULL,
            quantity TEXT,
            category TEXT,
            checked INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS habits (
            id TEXT PRIMARY KEY,
            profile_id TEXT NOT NULL REFERENCES profiles(id),
            name TEXT NOT NULL,
            frequency TEXT NOT NULL DEFAULT 'daily' CHECK(frequency IN ('daily', 'weekly')),
            target INTEGER,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS habit_logs (
            habit_id TEXT NOT NULL REFERENCES habits(id),
            date TEXT NOT NULL,
            completed INTEGER NOT NULL DEFAULT 1,
            PRIMARY KEY (habit_id, date)
        );

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        ",
    )?;

    // Seed default settings
    crate::db::repos::settings::SettingsRepo::seed_defaults(conn)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// After running migrations, the `messages` table must contain the new
    /// enrichment columns (tokens_count, collapsed_content, etc.).
    #[test]
    fn test_messages_table_has_new_columns() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        let mut stmt = conn.prepare("PRAGMA table_info(messages)").unwrap();
        let column_names: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

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
