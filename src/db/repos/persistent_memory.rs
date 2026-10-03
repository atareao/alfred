use sqlx::{Row, SqlitePool};

use crate::models::PersistentMemory;
use crate::persistent_memory::GLOBAL_STATE_ID;

/// Repository for the Capa C `persistent_memory` table: a single logical row
/// (`id = 'global_state'`) holding the versioned JSON state.
pub struct PersistentMemoryRepo;

impl PersistentMemoryRepo {
    /// Read the single persistent-memory row.
    ///
    /// The **absence** of the row is not an error: it means "empty state" and
    /// yields `None`. Reading never creates the row.
    pub async fn get(pool: &SqlitePool) -> Result<Option<PersistentMemory>, sqlx::Error> {
        let row =
            sqlx::query("SELECT id, payload, updated_at FROM persistent_memory WHERE id = ?1")
                .bind(GLOBAL_STATE_ID)
                .fetch_optional(pool)
                .await?;

        Ok(row.map(|r| PersistentMemory {
            id: r.get(0),
            payload: r.get(1),
            updated_at: r.get(2),
        }))
    }

    /// Insert or update the single `'global_state'` row.
    ///
    /// `updated_at` is passed in already resolved by the caller (Rust owns the
    /// timestamp rule; see [`crate::persistent_memory`]).
    pub async fn upsert(
        pool: &SqlitePool,
        payload: &str,
        updated_at: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO persistent_memory (id, payload, updated_at) VALUES (?1, ?2, ?3) \
             ON CONFLICT(id) DO UPDATE SET payload = excluded.payload, updated_at = excluded.updated_at",
        )
        .bind(GLOBAL_STATE_ID)
        .bind(payload)
        .bind(updated_at)
        .execute(pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    async fn setup() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await
            .expect("in-memory pool");
        crate::db::schema::run_migrations(&pool)
            .await
            .expect("migrations");
        pool
    }

    async fn row_count(pool: &SqlitePool) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM persistent_memory")
            .fetch_one(pool)
            .await
            .expect("count")
    }

    /// 2.3 — reading an empty state returns `None` without error and without
    /// creating the row.
    #[tokio::test]
    async fn test_get_empty_state_returns_none_without_creating_row() {
        let pool = setup().await;

        let state = PersistentMemoryRepo::get(&pool)
            .await
            .expect("get on an empty table must not error");

        assert!(state.is_none(), "no row ⇒ empty state (None)");
        assert_eq!(row_count(&pool).await, 0, "reading must not create the row");
    }

    /// 2.3 — the upsert creates the `'global_state'` row and a later upsert
    /// updates the very same row (never a second one).
    #[tokio::test]
    async fn test_upsert_creates_then_updates_single_row() {
        let pool = setup().await;

        PersistentMemoryRepo::upsert(&pool, r#"{"schema_version":1}"#, "2026-09-01T10:00:00Z")
            .await
            .expect("first upsert");

        let first = PersistentMemoryRepo::get(&pool)
            .await
            .expect("get")
            .expect("row must exist after upsert");
        assert_eq!(first.id, GLOBAL_STATE_ID);
        assert_eq!(first.payload, r#"{"schema_version":1}"#);
        assert_eq!(first.updated_at, "2026-09-01T10:00:00Z");
        assert_eq!(row_count(&pool).await, 1, "exactly one row");

        PersistentMemoryRepo::upsert(
            &pool,
            r#"{"schema_version":1,"system_rules":["sé breve"]}"#,
            "2026-10-03T08:00:00Z",
        )
        .await
        .expect("second upsert");

        let second = PersistentMemoryRepo::get(&pool)
            .await
            .expect("get")
            .expect("row still exists");
        assert_eq!(second.id, GLOBAL_STATE_ID);
        assert_eq!(
            second.payload,
            r#"{"schema_version":1,"system_rules":["sé breve"]}"#
        );
        assert_eq!(second.updated_at, "2026-10-03T08:00:00Z");
        assert_eq!(
            row_count(&pool).await,
            1,
            "the second upsert must update the row, not add one"
        );
    }
}
