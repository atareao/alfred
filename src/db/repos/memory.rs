use sqlx::{Row, Sqlite, SqlitePool, Transaction};
use uuid::Uuid;

use crate::models::Memory;

/// Repository for the episodic `memory` + `vec_memory` tables.
pub struct MemoryRepo;

#[allow(unused_variables)]
impl MemoryRepo {
    /// Create a new episodic memory card.
    ///
    /// Generates a UUID `id` and a `created_at` timestamp automatically.
    /// Persists the row in both `memory` and `vec_memory` tables.
    pub async fn create(
        pool: &SqlitePool,
        content: &str,
        tokens_count: usize,
        metadata: &serde_json::Value,
    ) -> Result<Memory, sqlx::Error> {
        let mut tx = pool.begin().await?;
        let memory = Self::create_in_tx(&mut tx, content, tokens_count, metadata).await?;
        tx.commit().await?;
        Ok(memory)
    }

    /// Create a new episodic memory card inside an existing transaction.
    ///
    /// Identical to [`create`](Self::create) but participates in the caller's
    /// transaction, so the `memory` row can be committed atomically together
    /// with its `vec_memory` embedding (no orphan rows on failure).
    pub async fn create_in_tx(
        tx: &mut Transaction<'_, Sqlite>,
        content: &str,
        tokens_count: usize,
        metadata: &serde_json::Value,
    ) -> Result<Memory, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let metadata_str = serde_json::to_string(metadata).unwrap_or_else(|_| "{}".to_string());

        sqlx::query(
            "INSERT INTO memory (id, content, tokens_count, created_at, metadata) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(&id)
        .bind(content)
        .bind(tokens_count as i64)
        .bind(&now)
        .bind(&metadata_str)
        .execute(&mut **tx)
        .await?;

        Ok(Memory {
            id,
            content: content.to_string(),
            tokens_count,
            created_at: now,
            metadata: metadata.clone(),
        })
    }

    /// Find a memory card by its primary key.
    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Memory>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, content, tokens_count, created_at, metadata \
             FROM memory WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|r| {
            let metadata_str: String = r.get(4);
            let metadata: serde_json::Value =
                serde_json::from_str(&metadata_str).unwrap_or(serde_json::json!({}));
            Memory {
                id: r.get(0),
                content: r.get(1),
                tokens_count: r.get::<i64, _>(2) as usize,
                created_at: r.get(3),
                metadata,
            }
        }))
    }

    /// List memory cards with pagination, ordered by `created_at DESC`.
    ///
    /// Returns `(items, total_count)`.
    pub async fn list(
        pool: &SqlitePool,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<Memory>, i64), sqlx::Error> {
        let actual_limit = limit.clamp(1, 100);
        let actual_offset = offset.max(0);

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memory")
            .fetch_one(pool)
            .await?;

        let rows = sqlx::query(
            "SELECT id, content, tokens_count, created_at, metadata \
             FROM memory ORDER BY created_at DESC LIMIT ?1 OFFSET ?2",
        )
        .bind(actual_limit)
        .bind(actual_offset)
        .fetch_all(pool)
        .await?;

        let items: Vec<Memory> = rows
            .iter()
            .map(|r| {
                let metadata_str: String = r.get(4);
                let metadata: serde_json::Value =
                    serde_json::from_str(&metadata_str).unwrap_or(serde_json::json!({}));
                Memory {
                    id: r.get(0),
                    content: r.get(1),
                    tokens_count: r.get::<i64, _>(2) as usize,
                    created_at: r.get(3),
                    metadata,
                }
            })
            .collect();

        Ok((items, total))
    }

    /// Delete a memory card by id.
    ///
    /// Returns `true` if a row was actually deleted, `false` otherwise.
    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM memory WHERE id = ?1")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Search memory cards by cosine similarity of their embeddings.
    ///
    /// * `query_embedding` — the vector to search with (cosine distance).
    /// * `limit` — maximum number of results (clamped to [1, 100]).
    /// * `budget_tokens` — maximum cumulative `tokens_count` across returned
    ///   items. Once the running sum exceeds this budget, no further items
    ///   are included.
    ///
    /// Results are ordered by descending similarity (highest score first).
    pub async fn search_by_vector(
        pool: &SqlitePool,
        query_embedding: &[f32],
        limit: i64,
        budget_tokens: usize,
    ) -> Result<Vec<Memory>, sqlx::Error> {
        // Degenerate case: empty query → no results
        if query_embedding.is_empty() {
            return Ok(Vec::new());
        }

        let actual_limit = limit.clamp(1, 100);

        // Load all embeddings from vec_memory
        let rows = sqlx::query("SELECT id, embedding FROM vec_memory WHERE embedding IS NOT NULL")
            .fetch_all(pool)
            .await?;

        let mut candidates: Vec<(String, f64)> = Vec::new();
        for row in rows {
            let id: String = row.get(0);
            let emb_str: String = row.get(1);
            if let Ok(emb) = serde_json::from_str::<Vec<f32>>(&emb_str) {
                let score = cosine_similarity(query_embedding, &emb);
                candidates.push((id, score));
            }
        }

        // Sort by descending similarity
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        candidates.truncate(actual_limit as usize);

        // Fetch Memory objects for the candidate IDs and apply budget_tokens filter.
        // Iterate in similarity order; skip items whose tokens_count would exceed
        // the remaining budget.
        let mut results: Vec<Memory> = Vec::new();
        let mut running_tokens: usize = 0;
        for (id, _score) in candidates {
            if let Some(mem) = Self::find_by_id(pool, &id).await? {
                if running_tokens + mem.tokens_count > budget_tokens {
                    continue;
                }
                running_tokens += mem.tokens_count;
                results.push(mem);
            }
        }

        Ok(results)
    }
}

/// Compute cosine similarity between two vectors.
///
/// Returns 0.0 if either vector is empty or their lengths differ.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    (dot / (norm_a * norm_b)) as f64
}

// ════════════════════════════════════════════════════════════════════════════
// RED tests – these will fail (compile or panic with todo!) until the GREEN
// phase provides implementations.
// ════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::run_migrations;
    use sqlx::sqlite::SqlitePoolOptions;

    /// Create a single-connection in-memory pool and run all migrations.
    /// The `memory` and `vec_memory` tables are created by migration
    /// `20260926000003_episodic_memory.sql`.
    async fn setup_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("failed to create in-memory pool");
        run_migrations(&pool).await.expect("migrations failed");
        pool
    }

    // ─── create ──────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_create_memory_with_metadata() {
        let pool = setup_pool().await;

        let metadata = serde_json::json!({"tags": ["rust"]});
        let mem = MemoryRepo::create(&pool, "Test memory content", 150, &metadata)
            .await
            .expect("create should succeed");

        // UUID is generated
        assert!(!mem.id.is_empty(), "id should be a non-empty UUID");

        // created_at is populated
        assert!(!mem.created_at.is_empty(), "created_at should not be empty");

        // Exact token count
        assert_eq!(mem.tokens_count, 150, "tokens_count should match input");

        // Content is preserved
        assert_eq!(mem.content, "Test memory content");

        // Metadata round-trips correctly
        assert_eq!(mem.metadata, metadata, "metadata should round-trip");
    }

    // ─── find_by_id ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_find_by_id_found() {
        let pool = setup_pool().await;

        let created = MemoryRepo::create(&pool, "Find me", 10, &serde_json::json!({}))
            .await
            .expect("create should succeed");

        let found = MemoryRepo::find_by_id(&pool, &created.id)
            .await
            .expect("find_by_id should not error");

        assert!(found.is_some(), "should find the memory by id");
        let mem = found.unwrap();
        assert_eq!(mem.content, "Find me");
        assert_eq!(mem.tokens_count, 10);
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() {
        let pool = setup_pool().await;

        let found = MemoryRepo::find_by_id(&pool, "nonexistent-id")
            .await
            .expect("find_by_id should not error");

        assert!(found.is_none(), "should return None for missing id");
    }

    // ─── list ────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_list_pagination() {
        let pool = setup_pool().await;

        // Insert three memories with staggered delays to guarantee
        // deterministic created_at ordering.
        let _m1 = MemoryRepo::create(&pool, "Alpha", 10, &serde_json::json!({}))
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let m2 = MemoryRepo::create(&pool, "Beta", 20, &serde_json::json!({}))
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let m3 = MemoryRepo::create(&pool, "Gamma", 30, &serde_json::json!({}))
            .await
            .unwrap();

        let (items, total) = MemoryRepo::list(&pool, 2, 0)
            .await
            .expect("list should succeed");

        // limit=2 → 2 items
        assert_eq!(items.len(), 2, "should return at most `limit` items");

        // total = 3
        assert_eq!(total, 3, "total should reflect all rows");

        // Descending order by created_at: Gamma (latest) first, then Beta
        assert_eq!(
            items[0].id, m3.id,
            "first item should be the most recent (Gamma)"
        );
        assert_eq!(
            items[1].id, m2.id,
            "second item should be the second most recent (Beta)"
        );
    }

    // ─── delete ──────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_delete_found() {
        let pool = setup_pool().await;

        let mem = MemoryRepo::create(&pool, "To delete", 5, &serde_json::json!({}))
            .await
            .expect("create should succeed");

        let deleted = MemoryRepo::delete(&pool, &mem.id)
            .await
            .expect("delete should not error");
        assert!(deleted, "delete should return true when a row is removed");
    }

    #[tokio::test]
    async fn test_delete_not_found() {
        let pool = setup_pool().await;

        let deleted = MemoryRepo::delete(&pool, "nonexistent-id")
            .await
            .expect("delete should not error");
        assert!(!deleted, "delete should return false for missing id");
    }

    // ─── search_by_vector ────────────────────────────────────────────────────

    /// Helper: insert an embedding row directly into `vec_memory`.
    async fn insert_embedding(pool: &SqlitePool, id: &str, embedding: &[f32]) {
        let json = serde_json::to_string(embedding).expect("failed to serialize embedding");
        sqlx::query("INSERT INTO vec_memory (id, embedding) VALUES (?1, ?2)")
            .bind(id)
            .bind(&json)
            .execute(pool)
            .await
            .expect("failed to insert vec_memory row");
    }

    #[tokio::test]
    async fn test_search_by_vector_returns_most_similar_first() {
        let pool = setup_pool().await;

        // Two memories with clearly different vectors
        let mem_a = MemoryRepo::create(&pool, "A: x-axis", 100, &serde_json::json!({}))
            .await
            .unwrap();
        let mem_b = MemoryRepo::create(&pool, "B: z-axis", 50, &serde_json::json!({}))
            .await
            .unwrap();

        // Embeddings: mem_a → [1,0,0],  mem_b → [0,0,1]
        insert_embedding(&pool, &mem_a.id, &[1.0, 0.0, 0.0]).await;
        insert_embedding(&pool, &mem_b.id, &[0.0, 0.0, 1.0]).await;

        // Query close to [1,0,0] → mem_a should be most similar
        let results = MemoryRepo::search_by_vector(&pool, &[0.9, 0.1, 0.0], 10, 5000)
            .await
            .expect("search_by_vector should succeed");

        assert!(!results.is_empty(), "should return at least one result");
        assert_eq!(
            results[0].id, mem_a.id,
            "most similar memory should be mem_a (x-axis)"
        );
    }

    #[tokio::test]
    async fn test_search_by_vector_budget_tokens_excludes_expensive() {
        let pool = setup_pool().await;

        // One memory with tokens_count=100
        let mem = MemoryRepo::create(&pool, "Expensive memory", 100, &serde_json::json!({}))
            .await
            .unwrap();
        insert_embedding(&pool, &mem.id, &[1.0, 0.0, 0.0]).await;

        // Budget of 50 is less than tokens_count=100 → nothing returned
        let results = MemoryRepo::search_by_vector(&pool, &[0.9, 0.1, 0.0], 10, 50)
            .await
            .expect("search_by_vector should succeed");

        assert!(
            results.is_empty(),
            "budget_tokens=50 should exclude a memory with tokens_count=100"
        );
    }

    /// Sanity check: an empty query embedding returns an empty result set
    /// (no crash on degenerate input).
    #[tokio::test]
    async fn test_search_by_vector_empty_embedding() {
        let pool = setup_pool().await;

        let mem = MemoryRepo::create(&pool, "Some memory", 10, &serde_json::json!({}))
            .await
            .unwrap();
        insert_embedding(&pool, &mem.id, &[0.5, 0.5]).await;

        let results = MemoryRepo::search_by_vector(&pool, &[], 10, 5000)
            .await
            .expect("search_by_vector should handle empty query gracefully");

        // Degenerate case: an empty query vector produces no similarity above
        // any reasonable threshold → empty result set is acceptable.
        assert!(
            results.is_empty(),
            "empty query embedding should return no results"
        );
    }
}
