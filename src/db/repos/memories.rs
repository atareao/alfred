// DEPRECATED - This module will be removed in Phase 6.
// Use MemoryRepo (src/db/repos/memory.rs) instead.
// The legacy `memories` table no longer exists; the new `memory` table
// uses a different schema (episodic, without profile_id/category/source/embedding_id).

use sqlx::SqlitePool;

use crate::models::Memory;

/// ⚠️ DEPRECATED — will be removed in Phase 6.
///
/// All callers should migrate to [`crate::db::repos::memory::MemoryRepo`].
pub struct MemoriesRepo;

#[allow(unused_variables, dead_code)]
impl MemoriesRepo {
    pub async fn create(
        pool: &SqlitePool,
        profile_id: &str,
        content: &str,
        category: &str,
        source: &str,
    ) -> Result<Memory, sqlx::Error> {
        // Redirect to MemoryRepo, ignoring legacy fields
        crate::db::repos::memory::MemoryRepo::create(
            pool,
            content,
            0,
            &serde_json::json!({"profile_id": profile_id, "category": category, "source": source}),
        )
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Memory>, sqlx::Error> {
        crate::db::repos::memory::MemoryRepo::find_by_id(pool, id).await
    }

    pub async fn list(
        pool: &SqlitePool,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<Memory>, i64), sqlx::Error> {
        crate::db::repos::memory::MemoryRepo::list(pool, limit, offset).await
    }

    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
        crate::db::repos::memory::MemoryRepo::delete(pool, id).await
    }
}
