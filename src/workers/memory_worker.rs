use sqlx::SqlitePool;

pub struct MemoryConsolidator {
    db: SqlitePool,
}

impl MemoryConsolidator {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    /// Run nightly consolidation.
    /// Returns a summary of what was cleaned up.
    pub async fn consolidate(&self) -> Result<String, String> {
        let mut report = vec![];

        // 1. Clean orphan message_embeddings
        let orphaned =
            sqlx::query("DELETE FROM message_embeddings WHERE id NOT IN (SELECT id FROM messages)")
                .execute(&self.db)
                .await
                .map_err(|e| e.to_string())?
                .rows_affected();

        if orphaned > 0 {
            report.push(format!(
                "🧹 Eliminados {} embeddings huérfanos de mensajes",
                orphaned
            ));
        }

        // 2. Clean orphan memory_embeddings
        let orphaned_mem =
            sqlx::query("DELETE FROM memory_embeddings WHERE id NOT IN (SELECT id FROM memories)")
                .execute(&self.db)
                .await
                .map_err(|e| e.to_string())?
                .rows_affected();

        if orphaned_mem > 0 {
            report.push(format!(
                "🧹 Eliminados {} embeddings huérfanos de memorias",
                orphaned_mem
            ));
        }

        // 3. (Removed: conversations table no longer exists)

        if report.is_empty() {
            report.push("✅ Todo en orden — no hay datos que limpiar".to_string());
        }

        Ok(report.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::run_migrations;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    async fn setup() -> (SqlitePool, MemoryConsolidator) {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await
            .unwrap();
        run_migrations(&pool).await.unwrap();
        let consolidator = MemoryConsolidator::new(pool.clone());
        (pool, consolidator)
    }

    #[tokio::test]
    async fn test_consolidate_empty_db() {
        let (_, consolidator) = setup().await;
        let result = consolidator.consolidate().await.unwrap();
        assert!(result.contains("Todo en orden"));
    }

    #[tokio::test]
    async fn test_consolidate_cleans_orphaned_message_embeddings() {
        let (pool, consolidator) = setup().await;
        // Insert orphan message_embedding (no corresponding message)
        sqlx::query("INSERT INTO message_embeddings (id, embedding) VALUES ('orphan-msg', '[]')")
            .execute(&pool)
            .await
            .unwrap();

        let result = consolidator.consolidate().await.unwrap();
        assert!(result.contains("embeddings huérfanos"));

        // Verify it was deleted
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM message_embeddings WHERE id = 'orphan-msg'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_consolidate_cleans_orphaned_memory_embeddings() {
        let (pool, consolidator) = setup().await;
        sqlx::query("INSERT INTO memory_embeddings (id, embedding) VALUES ('orphan-mem', '[]')")
            .execute(&pool)
            .await
            .unwrap();

        let result = consolidator.consolidate().await.unwrap();
        assert!(result.contains("embeddings huérfanos"));

        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM memory_embeddings WHERE id = 'orphan-mem'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, 0);
    }
}
