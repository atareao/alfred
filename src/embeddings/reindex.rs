use crate::embeddings::provider::{EmbeddingError, EmbeddingProvider};
use sqlx::SqlitePool;

/// Outcome of a full re-index run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReindexReport {
    /// Number of rows read from `memory`.
    pub total: usize,
    /// Number of embeddings successfully regenerated and upserted.
    pub updated: usize,
    /// Number of rows whose embedding generation failed.
    pub failed: usize,
}

/// Regenerate the embedding of every row in `memory` and upsert it into
/// `vec_memory`.
///
/// A provider failure on an individual row is counted in
/// [`ReindexReport::failed`] and does not abort the run. A dimension mismatch
/// when `dimension` is `Some` aborts the whole operation with an error.
pub async fn reindex_all(
    pool: &SqlitePool,
    provider: &dyn EmbeddingProvider,
    dimension: Option<usize>,
) -> Result<ReindexReport, EmbeddingError> {
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT id, content FROM memory")
        .fetch_all(pool)
        .await
        .map_err(storage_error)?;

    let total = rows.len();
    let mut failed = 0usize;
    // (id, serialized embedding) pairs collected first, so that a dimension
    // mismatch aborts before any write reaches `vec_memory`.
    let mut upserts: Vec<(String, String)> = Vec::with_capacity(total);

    for (id, content) in rows {
        match provider.embed(&content).await {
            Ok(embedding) => {
                if let Some(expected) = dimension {
                    if embedding.len() != expected {
                        return Err(EmbeddingError::Api(format!(
                            "embedding dimension mismatch: expected {expected}, got {}",
                            embedding.len()
                        )));
                    }
                }

                let serialized = serde_json::to_string(&embedding)
                    .map_err(|e| EmbeddingError::Api(e.to_string()))?;
                upserts.push((id, serialized));
            }
            Err(e) => {
                tracing::warn!("Failed to embed memory '{id}': {e}");
                failed += 1;
            }
        }
    }

    let updated = upserts.len();

    // Apply every upsert inside a single transaction: either all of them land,
    // or none does, so `vec_memory` is never left half-updated.
    //
    // `vec0` does not implement `ON CONFLICT`/UPSERT (`UPSERT not implemented
    // for virtual table`), so replace by id with an `UPDATE`, falling back to an
    // `INSERT` when the row does not exist yet.
    let mut tx = pool.begin().await.map_err(storage_error)?;
    for (id, serialized) in &upserts {
        let updated = sqlx::query("UPDATE vec_memory SET embedding = vec_f32(?1) WHERE id = ?2")
            .bind(serialized)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(storage_error)?
            .rows_affected();

        if updated == 0 {
            sqlx::query("INSERT INTO vec_memory (id, embedding) VALUES (?1, vec_f32(?2))")
                .bind(id)
                .bind(serialized)
                .execute(&mut *tx)
                .await
                .map_err(storage_error)?;
        }
    }
    tx.commit().await.map_err(storage_error)?;

    Ok(ReindexReport {
        total,
        updated,
        failed,
    })
}

/// Outcome of a source reset ([`reset_memory_source`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResetReport {
    /// Messages whose `is_indexed` flag was cleared (and `summary_ref` nulled).
    pub messages_reset: u64,
    /// Rows removed from the `memory` (source-of-truth) table.
    pub memories_deleted: u64,
    /// Vectors removed from the `vec_memory` index.
    pub vectors_deleted: u64,
}

/// Reset the *source* from which the episodic index is rebuilt.
///
/// Messages are the original data; the `memory` cards are derived data. The
/// robust rebuild is therefore to re-archive from the messages rather than to
/// re-embed the existing cards: re-embedding would feed already-summarised text
/// — and any mistake in a previous LLM summary — back into the index. So this
/// drops the derived data and un-indexes the messages, letting the
/// `EpisodicMemoryWorker` re-archive from scratch:
///
/// 1. `UPDATE messages SET is_indexed = 0, summary_ref = NULL` — every message
///    becomes unindexed again.
/// 2. `DELETE FROM memory` — drop the derived cards.
/// 3. `DELETE FROM vec_memory` — drop the vector index.
///
/// # FTS / triggers
///
/// The `memory` table has **no** FTS index and **no** triggers: the legacy
/// `memories_fts` virtual table and its `memories_fts_{ai,ad,au}` triggers
/// belonged to the old `memories` table and were dropped together with it by
/// migration `20260926000003_episodic_memory.sql`. The only FTS triggers left
/// in the schema hang off `messages` and `notes`, so deleting `memory` cannot
/// leave an FTS index stale. Step 1's `UPDATE` on `messages` fires the
/// `messages_fts_au` AFTER UPDATE trigger, which keeps `messages_fts` in sync.
///
/// All three statements run in a single transaction, so the source and the
/// index are never observed half-reset.
pub async fn reset_memory_source(pool: &SqlitePool) -> Result<ResetReport, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let messages_reset = sqlx::query("UPDATE messages SET is_indexed = 0, summary_ref = NULL")
        .execute(&mut *tx)
        .await?
        .rows_affected();

    let memories_deleted = sqlx::query("DELETE FROM memory")
        .execute(&mut *tx)
        .await?
        .rows_affected();

    let vectors_deleted = sqlx::query("DELETE FROM vec_memory")
        .execute(&mut *tx)
        .await?
        .rows_affected();

    tx.commit().await?;

    Ok(ResetReport {
        messages_reset,
        memories_deleted,
        vectors_deleted,
    })
}

/// Map a sqlx error from the storage layer into [`EmbeddingError::Storage`].
fn storage_error(e: sqlx::Error) -> EmbeddingError {
    EmbeddingError::Storage(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embeddings::provider::{EmbeddingError, EmbeddingProvider};
    use async_trait::async_trait;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use sqlx::SqlitePool;

    struct MockProvider {
        vector: Vec<f32>,
        fail_on: Option<String>,
    }

    /// Pad a leading slice to the 1024 dimensions the `vec0` table declares.
    fn v1024(leading: &[f32]) -> Vec<f32> {
        let mut v = leading.to_vec();
        v.resize(1024, 0.0);
        v
    }

    #[async_trait]
    impl EmbeddingProvider for MockProvider {
        async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
            if self.fail_on.as_deref() == Some(text) {
                return Err(EmbeddingError::Api("mock failure".into()));
            }
            Ok(self.vector.clone())
        }
    }

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
        crate::db::schema::run_migrations(&pool).await.unwrap();
        pool
    }

    async fn insert_memory(pool: &SqlitePool, id: &str, content: &str) {
        sqlx::query("INSERT INTO memory (id, content, tokens_count) VALUES (?1, ?2, 0)")
            .bind(id)
            .bind(content)
            .execute(pool)
            .await
            .unwrap();
    }

    /// Re-indexing regenerates an embedding for every row and upserts it.
    #[tokio::test]
    async fn test_reindex_all_updates_all_rows() {
        let pool = setup().await;
        insert_memory(&pool, "m1", "one").await;
        insert_memory(&pool, "m2", "two").await;
        insert_memory(&pool, "m3", "three").await;

        let provider = MockProvider {
            vector: v1024(&[0.1, 0.2, 0.3]),
            fail_on: None,
        };
        let report = reindex_all(&pool, &provider, None).await.unwrap();

        assert_eq!(
            report,
            ReindexReport {
                total: 3,
                updated: 3,
                failed: 0
            }
        );
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM vec_memory")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 3);
    }

    /// Running the re-index twice overwrites embeddings without duplicating rows.
    #[tokio::test]
    async fn test_reindex_all_is_idempotent() {
        let pool = setup().await;
        insert_memory(&pool, "m1", "one").await;
        insert_memory(&pool, "m2", "two").await;

        let provider = MockProvider {
            vector: v1024(&[1.0, 0.0]),
            fail_on: None,
        };
        reindex_all(&pool, &provider, None).await.unwrap();
        reindex_all(&pool, &provider, None).await.unwrap();

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM vec_memory")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 2);
    }

    /// A provider error on one row is counted as failed and does not abort the run.
    #[tokio::test]
    async fn test_reindex_all_continues_after_provider_failure() {
        let pool = setup().await;
        insert_memory(&pool, "m1", "good").await;
        insert_memory(&pool, "m2", "bad").await;
        insert_memory(&pool, "m3", "good2").await;

        let provider = MockProvider {
            vector: v1024(&[0.5]),
            fail_on: Some("bad".into()),
        };
        let report = reindex_all(&pool, &provider, None).await.unwrap();

        assert_eq!(report.total, 3);
        assert_eq!(report.updated, 2);
        assert_eq!(report.failed, 1);

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM vec_memory")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 2);
    }

    /// A generated embedding whose length differs from `dimension` aborts with an error.
    #[tokio::test]
    async fn test_reindex_all_dimension_mismatch_is_error() {
        let pool = setup().await;
        insert_memory(&pool, "m1", "one").await;

        let provider = MockProvider {
            vector: v1024(&[0.1, 0.2, 0.3]),
            fail_on: None,
        };
        let result = reindex_all(&pool, &provider, Some(1536)).await;
        assert!(matches!(result, Err(EmbeddingError::Api(_))));
    }

    /// A dimension mismatch must abort before writing anything: an existing
    /// `vec_memory` row is left untouched.
    ///
    /// CHANGED ON PURPOSE (invariant exception 2): the row is now seeded
    /// through `vec_f32(?)` (a valid 1024-dim vector) and compared by bytes,
    /// because `vec0` stores binary vectors instead of JSON text. The
    /// assertion (unchanged contents on abort) is the same.
    #[tokio::test]
    async fn test_reindex_all_dimension_mismatch_does_not_modify_vec_memory() {
        let pool = setup().await;
        insert_memory(&pool, "m1", "one").await;

        let original = v1024(&[9.0]);
        let original_json = serde_json::to_string(&original).unwrap();
        sqlx::query("INSERT INTO vec_memory (id, embedding) VALUES ('m1', vec_f32(?1))")
            .bind(&original_json)
            .execute(&pool)
            .await
            .unwrap();

        let provider = MockProvider {
            vector: v1024(&[0.1, 0.2, 0.3]),
            fail_on: None,
        };
        let result = reindex_all(&pool, &provider, Some(1536)).await;
        assert!(matches!(result, Err(EmbeddingError::Api(_))));

        let existing: Vec<u8> =
            sqlx::query_scalar("SELECT embedding FROM vec_memory WHERE id = 'm1'")
                .fetch_one(&pool)
                .await
                .unwrap();
        let expected: Vec<u8> = original.iter().flat_map(|f| f.to_le_bytes()).collect();
        assert_eq!(
            existing, expected,
            "vec_memory must not be modified on abort"
        );
    }
}
