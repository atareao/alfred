use chrono::Utc;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::models::Memory;

pub struct MemoriesRepo;

impl MemoriesRepo {
    pub async fn create(
        pool: &SqlitePool,
        profile_id: &str,
        content: &str,
        category: &str,
        source: &str,
    ) -> Result<Memory, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO memories (id, profile_id, content, category, source, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(&id)
        .bind(profile_id)
        .bind(content)
        .bind(category)
        .bind(source)
        .bind(&now)
        .execute(pool)
        .await?;

        Ok(Memory {
            id,
            profile_id: profile_id.to_string(),
            content: content.to_string(),
            category: category.to_string(),
            source: source.to_string(),
            embedding_id: None,
            created_at: now,
        })
    }

    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Memory>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, profile_id, content, category, source, embedding_id, created_at \
             FROM memories WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|r| Memory {
            id: r.get(0),
            profile_id: r.get(1),
            content: r.get(2),
            category: r.get(3),
            source: r.get(4),
            embedding_id: r.get(5),
            created_at: r.get(6),
        }))
    }

    pub async fn list(
        pool: &SqlitePool,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<Memory>, i64), sqlx::Error> {
        let actual_limit = limit.clamp(1, 100);
        let actual_offset = offset.max(0);

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memories")
            .fetch_one(pool)
            .await?;

        let rows = sqlx::query(
            "SELECT id, profile_id, content, category, source, embedding_id, created_at \
             FROM memories ORDER BY created_at DESC LIMIT ?1 OFFSET ?2",
        )
        .bind(actual_limit)
        .bind(actual_offset)
        .fetch_all(pool)
        .await?;

        let items: Vec<Memory> = rows
            .iter()
            .map(|r| Memory {
                id: r.get(0),
                profile_id: r.get(1),
                content: r.get(2),
                category: r.get(3),
                source: r.get(4),
                embedding_id: r.get(5),
                created_at: r.get(6),
            })
            .collect();

        Ok((items, total))
    }

    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM memories WHERE id = ?1")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;
    use uuid::Uuid;

    /// Creates a single-connection in-memory pool with the minimal schema
    /// needed for memory tests and a seed profile row.
    async fn setup_pool() -> Result<(SqlitePool, String), sqlx::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;

        // ── Profiles table ──────────────────────────────────────────────────
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS profiles (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                avatar_url TEXT,
                preferences TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
        )
        .execute(&pool)
        .await?;

        // ── Memories table ──────────────────────────────────────────────────
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                profile_id TEXT NOT NULL REFERENCES profiles(id),
                content TEXT NOT NULL,
                category TEXT NOT NULL DEFAULT 'general',
                source TEXT NOT NULL DEFAULT 'manual',
                embedding_id TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
        )
        .execute(&pool)
        .await?;

        // Seed a default profile
        let profile_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO profiles (id, name, preferences, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(&profile_id)
        .bind("Alfred User")
        .bind("{}")
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await?;

        Ok((pool, profile_id))
    }

    #[tokio::test]
    async fn test_create_memory() -> Result<(), Box<dyn std::error::Error>> {
        let (pool, profile_id) = setup_pool().await?;
        let mem =
            MemoriesRepo::create(&pool, &profile_id, "Remember this", "fact", "manual").await?;
        assert!(!mem.id.is_empty());
        assert_eq!(mem.content, "Remember this");
        assert_eq!(mem.category, "fact");

        Ok(())
    }

    #[tokio::test]
    async fn test_create_memory_default_category() -> Result<(), Box<dyn std::error::Error>> {
        let (pool, profile_id) = setup_pool().await?;
        let mem =
            MemoriesRepo::create(&pool, &profile_id, "Default cat", "general", "chat").await?;
        assert_eq!(mem.category, "general");
        assert_eq!(mem.source, "chat");

        Ok(())
    }

    #[tokio::test]
    async fn test_find_by_id_found() -> Result<(), Box<dyn std::error::Error>> {
        let (pool, profile_id) = setup_pool().await?;
        let created = MemoriesRepo::create(&pool, &profile_id, "Find me", "fact", "manual").await?;
        let found = MemoriesRepo::find_by_id(&pool, &created.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().content, "Find me");

        Ok(())
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;

        // Create minimal tables (just memories — no FK enforcement at runtime
        // by default in sqlx/SQLite, so we can query directly).
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                profile_id TEXT NOT NULL,
                content TEXT NOT NULL,
                category TEXT NOT NULL DEFAULT 'general',
                source TEXT NOT NULL DEFAULT 'manual',
                embedding_id TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
        )
        .execute(&pool)
        .await?;

        let found = MemoriesRepo::find_by_id(&pool, "nonexistent").await?;
        assert!(found.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_list_memories() -> Result<(), Box<dyn std::error::Error>> {
        let (pool, profile_id) = setup_pool().await?;
        MemoriesRepo::create(&pool, &profile_id, "First", "general", "manual").await?;
        MemoriesRepo::create(&pool, &profile_id, "Second", "fact", "manual").await?;
        let (data, total) = MemoriesRepo::list(&pool, 10, 0).await.unwrap();
        assert_eq!(data.len(), 2);
        assert_eq!(total, 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_list_memories_with_pagination() -> Result<(), Box<dyn std::error::Error>> {
        let (pool, profile_id) = setup_pool().await?;
        for i in 0..5 {
            MemoriesRepo::create(
                &pool,
                &profile_id,
                &format!("Memory {}", i),
                "general",
                "manual",
            )
            .await?;
        }
        let (page1, total) = MemoriesRepo::list(&pool, 2, 0).await.unwrap();
        assert_eq!(page1.len(), 2);
        assert_eq!(total, 5);

        let (page2, _) = MemoriesRepo::list(&pool, 2, 2).await.unwrap();
        assert_eq!(page2.len(), 2);

        let (page3, _) = MemoriesRepo::list(&pool, 2, 4).await.unwrap();
        assert_eq!(page3.len(), 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_found() -> Result<(), Box<dyn std::error::Error>> {
        let (pool, profile_id) = setup_pool().await?;
        let mem =
            MemoriesRepo::create(&pool, &profile_id, "To delete", "general", "manual").await?;
        assert!(MemoriesRepo::delete(&pool, &mem.id).await.unwrap());

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                profile_id TEXT NOT NULL,
                content TEXT NOT NULL,
                category TEXT NOT NULL DEFAULT 'general',
                source TEXT NOT NULL DEFAULT 'manual',
                embedding_id TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
        )
        .execute(&pool)
        .await?;

        assert!(!MemoriesRepo::delete(&pool, "nonexistent").await.unwrap());

        Ok(())
    }
}
