use rusqlite::{params, Connection};

/// Register sqlite-vec extension. Gracefully degrades if not available.
pub fn register_vector_ext(_conn: &Connection) -> Result<(), String> {
    #[cfg(not(feature = "vec0"))]
    {
        tracing::info!("sqlite-vec feature disabled");
    }
    #[cfg(feature = "vec0")]
    {
        unsafe {
            _conn.load_extension_enable().map_err(|e| e.to_string())?;
        }
        let result = _conn.execute_batch("SELECT load_extension('vec0');");
        unsafe {
            _conn.load_extension_disable().map_err(|e| e.to_string())?;
        }
        if let Err(e) = result {
            tracing::warn!(
                "sqlite-vec not available ({}), falling back to FTS5 only",
                e
            );
        } else {
            tracing::info!("sqlite-vec registered successfully");
        }
    }
    Ok(())
}

/// Store an embedding vector for a message
pub fn store_message_embedding(
    conn: &Connection,
    message_id: &str,
    embedding: &[f32],
) -> Result<(), rusqlite::Error> {
    let emb_json = serde_json::to_string(embedding).unwrap_or_default();
    conn.execute(
        "INSERT OR REPLACE INTO message_embeddings (id, embedding) VALUES (?1, ?2)",
        params![message_id, emb_json],
    )?;
    Ok(())
}

/// Store an embedding vector for a memory
pub fn store_memory_embedding(
    conn: &Connection,
    memory_id: &str,
    embedding: &[f32],
) -> Result<(), rusqlite::Error> {
    let emb_json = serde_json::to_string(embedding).unwrap_or_default();
    conn.execute(
        "INSERT OR REPLACE INTO memory_embeddings (id, embedding) VALUES (?1, ?2)",
        params![memory_id, emb_json],
    )?;
    Ok(())
}

/// Search message vectors by cosine similarity
pub fn search_message_vectors(
    conn: &Connection,
    query_embedding: &[f32],
    limit: i64,
) -> Result<Vec<(String, f64)>, rusqlite::Error> {
    let actual_limit = limit.clamp(1, 100);
    let mut stmt =
        conn.prepare("SELECT id, embedding FROM message_embeddings WHERE embedding IS NOT NULL")?;
    let mut rows = stmt.query([])?;

    let mut results: Vec<(String, f64)> = Vec::new();
    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let emb_str: String = row.get(1)?;
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
pub fn search_memory_vectors(
    conn: &Connection,
    query_embedding: &[f32],
    limit: i64,
) -> Result<Vec<(String, f64)>, rusqlite::Error> {
    let actual_limit = limit.clamp(1, 100);
    let mut stmt =
        conn.prepare("SELECT id, embedding FROM memory_embeddings WHERE embedding IS NOT NULL")?;
    let mut rows = stmt.query([])?;

    let mut results: Vec<(String, f64)> = Vec::new();
    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let emb_str: String = row.get(1)?;
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

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS message_embeddings (id TEXT PRIMARY KEY, embedding TEXT);
             CREATE TABLE IF NOT EXISTS memory_embeddings (id TEXT PRIMARY KEY, embedding TEXT);",
        )
        .unwrap();
        conn
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

    #[test]
    fn test_store_and_search_message() {
        let conn = setup();
        store_message_embedding(&conn, "msg1", &[1.0, 0.0, 0.0]).unwrap();

        let results = search_message_vectors(&conn, &[1.0, 0.0, 0.0], 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "msg1");
        assert!((results[0].1 - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_search_orders_by_similarity() {
        let conn = setup();
        store_message_embedding(&conn, "close", &[1.0, 0.0, 0.0]).unwrap();
        store_message_embedding(&conn, "far", &[0.0, 0.0, 1.0]).unwrap();

        let results = search_message_vectors(&conn, &[0.9, 0.1, 0.0], 10).unwrap();
        assert_eq!(results[0].0, "close");
    }

    #[test]
    fn test_empty_embedding_returns_empty() {
        let conn = setup();
        let results = search_message_vectors(&conn, &[1.0, 0.0], 10).unwrap();
        assert!(results.is_empty());
    }
}
