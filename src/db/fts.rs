use sqlx::{Row, SqlitePool};

/// Add FTS5 triggers to keep indexes up to date
pub async fn create_fts_triggers(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // Messages FTS triggers
    sqlx::query(
        "CREATE TRIGGER IF NOT EXISTS messages_fts_ai AFTER INSERT ON messages BEGIN
            INSERT INTO messages_fts(rowid, content) VALUES (new.rowid, new.content);
        END;",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TRIGGER IF NOT EXISTS messages_fts_ad AFTER DELETE ON messages BEGIN
            INSERT INTO messages_fts(messages_fts, rowid, content) VALUES('delete', old.rowid, old.content);
        END;",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TRIGGER IF NOT EXISTS messages_fts_au AFTER UPDATE ON messages BEGIN
            INSERT INTO messages_fts(messages_fts, rowid, content) VALUES('delete', old.rowid, old.content);
            INSERT INTO messages_fts(rowid, content) VALUES (new.rowid, new.content);
        END;",
    )
    .execute(pool)
    .await?;

    // Memories FTS triggers
    sqlx::query(
        "CREATE TRIGGER IF NOT EXISTS memories_fts_ai AFTER INSERT ON memories BEGIN
            INSERT INTO memories_fts(rowid, content) VALUES (new.rowid, new.content);
        END;",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TRIGGER IF NOT EXISTS memories_fts_ad AFTER DELETE ON memories BEGIN
            INSERT INTO memories_fts(memories_fts, rowid, content) VALUES('delete', old.rowid, old.content);
        END;",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TRIGGER IF NOT EXISTS memories_fts_au AFTER UPDATE ON memories BEGIN
            INSERT INTO memories_fts(memories_fts, rowid, content) VALUES('delete', old.rowid, old.content);
            INSERT INTO memories_fts(rowid, content) VALUES (new.rowid, new.content);
        END;",
    )
    .execute(pool)
    .await?;

    // Notes FTS triggers
    sqlx::query(
        "CREATE TRIGGER IF NOT EXISTS notes_ai AFTER INSERT ON notes BEGIN
            INSERT INTO notes_fts(rowid, content) VALUES (new.rowid, new.content);
        END;",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TRIGGER IF NOT EXISTS notes_ad AFTER DELETE ON notes BEGIN
            INSERT INTO notes_fts(notes_fts, rowid, content) VALUES('delete', old.rowid, old.content);
        END;",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TRIGGER IF NOT EXISTS notes_au AFTER UPDATE ON notes BEGIN
            INSERT INTO notes_fts(notes_fts, rowid, content) VALUES('delete', old.rowid, old.content);
            INSERT INTO notes_fts(rowid, content) VALUES (new.rowid, new.content);
        END;",
    )
    .execute(pool)
    .await?;

    tracing::info!("FTS5 triggers created");
    Ok(())
}

/// Search messages using FTS5
pub async fn search_messages_fts(
    pool: &SqlitePool,
    query: &str,
    limit: i64,
) -> Result<Vec<(String, String, f64)>, sqlx::Error> {
    let actual_limit = limit.clamp(1, 100);
    // Sanitize FTS5 query: escape special chars and add prefix matching
    let sanitized: String = query
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect();
    if sanitized.trim().is_empty() {
        return Ok(Vec::new());
    }
    // Use simple prefix matching
    let fts_query = sanitized
        .split_whitespace()
        .map(|w| format!("{}*", w))
        .collect::<Vec<_>>()
        .join(" AND ");

    let rows = sqlx::query(
        "SELECT m.id, m.content, rank
         FROM messages_fts
         JOIN messages m ON messages_fts.rowid = m.rowid
         WHERE messages_fts MATCH ?1
         ORDER BY rank
         LIMIT ?2",
    )
    .bind(&fts_query)
    .bind(actual_limit)
    .fetch_all(pool)
    .await?;

    let results: Vec<(String, String, f64)> = rows
        .iter()
        .map(|row| {
            (
                row.get::<String, _>(0),
                row.get::<String, _>(1),
                row.get::<f64, _>(2),
            )
        })
        .collect();
    Ok(results)
}

/// Search memories using FTS5
pub async fn search_memories_fts(
    pool: &SqlitePool,
    query: &str,
    limit: i64,
) -> Result<Vec<(String, String, f64)>, sqlx::Error> {
    let actual_limit = limit.clamp(1, 100);
    let sanitized: String = query
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect();
    if sanitized.trim().is_empty() {
        return Ok(Vec::new());
    }
    let fts_query = sanitized
        .split_whitespace()
        .map(|w| format!("{}*", w))
        .collect::<Vec<_>>()
        .join(" AND ");

    let rows = sqlx::query(
        "SELECT m.id, m.content, rank
         FROM memories_fts
         JOIN memories m ON memories_fts.rowid = m.rowid
         WHERE memories_fts MATCH ?1
         ORDER BY rank
         LIMIT ?2",
    )
    .bind(&fts_query)
    .bind(actual_limit)
    .fetch_all(pool)
    .await?;

    let results: Vec<(String, String, f64)> = rows
        .iter()
        .map(|row| {
            (
                row.get::<String, _>(0),
                row.get::<String, _>(1),
                row.get::<f64, _>(2),
            )
        })
        .collect();
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;
    use sqlx::SqlitePool;

    async fn setup_pool() -> Result<SqlitePool, sqlx::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                sqlx::sqlite::SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await?;
        sqlx::migrate::Migrator::new(std::path::Path::new("migrations"))
            .await
            .unwrap()
            .run(&pool)
            .await
            .unwrap();
        Ok(pool)
    }

    #[tokio::test]
    async fn test_trigger_inserts_into_fts() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup_pool().await?;
        // Insert a message directly (no conversation FK needed)
        sqlx::query("INSERT INTO messages (id, role, content) VALUES (?1, ?2, ?3)")
            .bind("m1")
            .bind("user")
            .bind("receta de pasta")
            .execute(&pool)
            .await?;

        let results = search_messages_fts(&pool, "receta", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "m1");
        Ok(())
    }

    #[tokio::test]
    async fn test_trigger_deletes_from_fts() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup_pool().await?;
        // Insert a message directly (no conversation FK needed)
        sqlx::query("INSERT INTO messages (id, role, content) VALUES (?1, ?2, ?3)")
            .bind("m1")
            .bind("user")
            .bind("test content")
            .execute(&pool)
            .await?;

        sqlx::query("DELETE FROM messages WHERE id = ?1")
            .bind("m1")
            .execute(&pool)
            .await?;

        let results = search_messages_fts(&pool, "test", 10).await.unwrap();
        assert_eq!(results.len(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn test_search_memories_fts() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup_pool().await?;
        // Create a profile first to satisfy FK constraint
        sqlx::query(
            "INSERT INTO profiles (id, name, preferences, created_at, updated_at) VALUES ('p1', 'Test', '{}', datetime('now'), datetime('now'))"
        )
        .execute(&pool)
        .await?;
        sqlx::query(
            "INSERT INTO memories (id, profile_id, content, category, source) VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind("mem1")
        .bind("p1")
        .bind("A Alfred le gusta el café")
        .bind("fact")
        .bind("manual")
        .execute(&pool)
        .await?;

        let results = search_memories_fts(&pool, "café", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_empty_query_returns_empty() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup_pool().await?;
        let results = search_messages_fts(&pool, "", 10).await.unwrap();
        assert!(results.is_empty());
        Ok(())
    }
}
