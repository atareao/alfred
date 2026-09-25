use sqlx::{Row, SqlitePool};

/// Store an embedding vector for a message
pub async fn store_message_embedding(
    pool: &SqlitePool,
    message_id: &str,
    embedding: &[f32],
) -> Result<(), sqlx::Error> {
    let emb_json = serde_json::to_string(embedding).unwrap_or_default();
    sqlx::query("INSERT OR REPLACE INTO message_embeddings (id, embedding) VALUES (?1, ?2)")
        .bind(message_id)
        .bind(&emb_json)
        .execute(pool)
        .await?;
    Ok(())
}

/// Store an embedding vector for a memory
pub async fn store_memory_embedding(
    pool: &SqlitePool,
    memory_id: &str,
    embedding: &[f32],
) -> Result<(), sqlx::Error> {
    let emb_json = serde_json::to_string(embedding).unwrap_or_default();
    sqlx::query("INSERT OR REPLACE INTO memory_embeddings (id, embedding) VALUES (?1, ?2)")
        .bind(memory_id)
        .bind(&emb_json)
        .execute(pool)
        .await?;
    Ok(())
}

/// Search message vectors by cosine similarity
pub async fn search_message_vectors(
    pool: &SqlitePool,
    query_embedding: &[f32],
    limit: i64,
) -> Result<Vec<(String, f64)>, sqlx::Error> {
    let actual_limit = limit.clamp(1, 100);
    let rows =
        sqlx::query("SELECT id, embedding FROM message_embeddings WHERE embedding IS NOT NULL")
            .fetch_all(pool)
            .await?;

    let mut results: Vec<(String, f64)> = Vec::new();
    for row in rows {
        let id: String = row.get(0);
        let emb_str: String = row.get(1);
        if let Ok(emb) = serde_json::from_str::<Vec<f32>>(&emb_str) {
            let score = cosine_similarity(query_embedding, &emb);
            results.push((id, score));
        }
    }

    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(actual_limit as usize);
    Ok(results)
}

/// Search memory vectors by cosine similarity
pub async fn search_memory_vectors(
    pool: &SqlitePool,
    query_embedding: &[f32],
    limit: i64,
) -> Result<Vec<(String, f64)>, sqlx::Error> {
    let actual_limit = limit.clamp(1, 100);
    let rows =
        sqlx::query("SELECT id, embedding FROM memory_embeddings WHERE embedding IS NOT NULL")
            .fetch_all(pool)
            .await?;

    let mut results: Vec<(String, f64)> = Vec::new();
    for row in rows {
        let id: String = row.get(0);
        let emb_str: String = row.get(1);
        if let Ok(emb) = serde_json::from_str::<Vec<f32>>(&emb_str) {
            let score = cosine_similarity(query_embedding, &emb);
            results.push((id, score));
        }
    }

    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(actual_limit as usize);
    Ok(results)
}

/// Compute cosine similarity between two vectors
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

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;
    use sqlx::SqlitePool;

    async fn setup() -> Result<SqlitePool, sqlx::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                sqlx::sqlite::SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await?;
        // Create embedding tables directly (no need for full migration)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS message_embeddings (id TEXT PRIMARY KEY, embedding TEXT);",
        )
        .execute(&pool)
        .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS memory_embeddings (id TEXT PRIMARY KEY, embedding TEXT);",
        )
        .execute(&pool)
        .await?;
        Ok(pool)
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let score = cosine_similarity(&a, &a);
        assert!((score - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let score = cosine_similarity(&a, &b);
        assert!((score - 0.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_store_and_search_message() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        store_message_embedding(&pool, "msg1", &[1.0, 0.0, 0.0]).await?;

        let results = search_message_vectors(&pool, &[1.0, 0.0, 0.0], 10).await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "msg1");
        assert!((results[0].1 - 1.0).abs() < 0.001);
        Ok(())
    }

    #[tokio::test]
    async fn test_search_orders_by_similarity() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        store_message_embedding(&pool, "close", &[1.0, 0.0, 0.0]).await?;
        store_message_embedding(&pool, "far", &[0.0, 0.0, 1.0]).await?;

        let results = search_message_vectors(&pool, &[0.9, 0.1, 0.0], 10).await?;
        assert_eq!(results[0].0, "close");
        Ok(())
    }

    #[tokio::test]
    async fn test_empty_embedding_returns_empty() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        let results = search_message_vectors(&pool, &[1.0, 0.0], 10).await?;
        assert!(results.is_empty());
        Ok(())
    }
}
