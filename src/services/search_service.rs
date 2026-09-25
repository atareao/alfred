use serde::Serialize;
use sqlx::{Row, SqlitePool};
use std::cmp::Ordering;

use crate::db::fts;
use crate::db::vector;
use crate::embeddings::EmbeddingProvider;

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub id: String,
    pub content: String,
    pub score: f64,
    pub source: String,
    pub created_at: String,
}

pub enum SearchType {
    Messages,
    Memories,
    All,
}

impl SearchType {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "message" | "messages" => Self::Messages,
            "memory" | "memories" => Self::Memories,
            _ => Self::All,
        }
    }
}

pub struct SearchService {
    provider: Box<dyn EmbeddingProvider>,
}

impl SearchService {
    pub fn new(provider: Box<dyn EmbeddingProvider>) -> Self {
        Self { provider }
    }

    /// Hybrid search: vector + FTS5 with Reciprocal Rank Fusion
    pub async fn search(
        &self,
        pool: &SqlitePool,
        query: &str,
        search_type: SearchType,
        limit: i64,
    ) -> Result<Vec<SearchResult>, String> {
        if query.trim().is_empty() {
            return Err("Query cannot be empty".to_string());
        }

        let actual_limit = limit.clamp(1, 100);
        let k = 60.0_f64; // RRF constant

        // Get FTS5 results
        let fts_results = match &search_type {
            SearchType::Messages | SearchType::All => {
                fts::search_messages_fts(pool, query, actual_limit * 2)
                    .await
                    .map_err(|e| format!("FTS5 search error: {}", e))?
                    .into_iter()
                    .map(|(id, content, _)| (id, content, "message".to_string()))
                    .collect::<Vec<_>>()
            }
            _ => Vec::new(),
        };

        let fts_memories = match &search_type {
            SearchType::Memories | SearchType::All => {
                fts::search_memories_fts(pool, query, actual_limit * 2)
                    .await
                    .map_err(|e| format!("FTS5 search error: {}", e))?
                    .into_iter()
                    .map(|(id, content, _)| (id, content, "memory".to_string()))
                    .collect::<Vec<_>>()
            }
            _ => Vec::new(),
        };

        // Try vector search
        let vector_results = match self.provider.embed(query).await {
            Ok(embedding) => {
                let msg_vec = match &search_type {
                    SearchType::Messages | SearchType::All => {
                        vector::search_message_vectors(pool, &embedding, actual_limit * 2)
                            .await
                            .map_err(|e| format!("Vector search error: {}", e))?
                            .into_iter()
                            .map(|(id, score)| (id, score, "message".to_string()))
                            .collect::<Vec<_>>()
                    }
                    _ => Vec::new(),
                };
                let mem_vec = match &search_type {
                    SearchType::Memories | SearchType::All => {
                        vector::search_memory_vectors(pool, &embedding, actual_limit * 2)
                            .await
                            .map_err(|e| format!("Vector search error: {}", e))?
                            .into_iter()
                            .map(|(id, score)| (id, score, "memory".to_string()))
                            .collect::<Vec<_>>()
                    }
                    _ => Vec::new(),
                };
                let mut all = msg_vec;
                all.extend(mem_vec);
                all
            }
            Err(e) => {
                tracing::warn!("Vector search unavailable ({}), using FTS5 only", e);
                Vec::new()
            }
        };

        // RRF: combine FTS5 and vector results
        let mut score_map: Vec<(String, f64, String)> = Vec::new(); // (id, score, source)

        // Score FTS5 results for messages
        for (rank, (id, _, source)) in fts_results.iter().enumerate() {
            let rrf_score = 1.0 / (k + rank as f64);
            score_map.push((id.clone(), rrf_score, source.clone()));
        }

        // Score FTS5 results for memories
        for (rank, (id, _, source)) in fts_memories.iter().enumerate() {
            let rrf_score = 1.0 / (k + rank as f64);
            score_map.push((id.clone(), rrf_score, source.clone()));
        }

        // Score vector results (merge with existing)
        for (vid, _, source) in &vector_results {
            if let Some(existing) = score_map
                .iter_mut()
                .find(|(id, _, s)| id == vid && s == source)
            {
                existing.1 += 1.0 / (k + 1.0); // vector rank ~1 since we used similarity
            } else {
                score_map.push((vid.clone(), 1.0 / (k + 1.0), source.clone()));
            }
        }

        // Sort by score descending
        score_map.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        score_map.truncate(actual_limit as usize);

        // Build results with content and created_at
        let mut results = Vec::with_capacity(score_map.len());
        for (id, score, source) in score_map {
            let content = get_content_by_id(&fts_results, &fts_memories, &id);
            let created_at = get_created_at(pool, &id, &source)
                .await
                .map_err(|e| format!("Failed to get created_at: {}", e))?;
            results.push(SearchResult {
                id,
                content,
                score,
                source,
                created_at,
            });
        }

        Ok(results)
    }
}

/// Helper to get content by id from FTS results
fn get_content_by_id(
    fts_results: &[(String, String, String)],
    fts_memories: &[(String, String, String)],
    id: &str,
) -> String {
    for (fid, content, _) in fts_results {
        if fid == id {
            return content.clone();
        }
    }
    for (fid, content, _) in fts_memories {
        if fid == id {
            return content.clone();
        }
    }
    String::new()
}

/// Helper to get created_at from the database
async fn get_created_at(pool: &SqlitePool, id: &str, source: &str) -> Result<String, String> {
    let table = if source == "message" {
        "messages"
    } else {
        "memories"
    };
    let query_str = format!("SELECT created_at FROM {} WHERE id = ?1", table);
    sqlx::query(&query_str)
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("DB query error: {}", e))?
        .map(|r| r.get::<String, _>(0))
        .ok_or_else(|| format!("No {} found with id {}", source, id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::fts as fts_mod;
    use crate::db::schema;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use sqlx::SqlitePool;

    async fn setup() -> Result<SqlitePool, sqlx::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await?;
        schema::run_migrations(&pool).await.unwrap();
        fts_mod::create_fts_triggers(&pool).await.unwrap();
        Ok(pool)
    }

    #[test]
    fn test_search_type_from_str() {
        assert!(matches!(
            SearchType::from_str("message"),
            SearchType::Messages
        ));
        assert!(matches!(
            SearchType::from_str("messages"),
            SearchType::Messages
        ));
        assert!(matches!(
            SearchType::from_str("memory"),
            SearchType::Memories
        ));
        assert!(matches!(
            SearchType::from_str("memories"),
            SearchType::Memories
        ));
        assert!(matches!(SearchType::from_str("all"), SearchType::All));
        assert!(matches!(SearchType::from_str("unknown"), SearchType::All));
    }

    #[tokio::test]
    async fn test_search_empty_query_returns_error() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        let provider =
            crate::embeddings::create_provider(&crate::embeddings::EmbeddingConfig::default());
        let service = SearchService::new(provider);

        let result = service.search(&pool, "", SearchType::All, 10).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Query cannot be empty");
        Ok(())
    }

    #[tokio::test]
    async fn test_search_fts5_returns_results() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        // Insert a message directly (no conversation FK needed)
        sqlx::query("INSERT INTO messages (id, role, content) VALUES (?1, ?2, ?3)")
            .bind("m1")
            .bind("user")
            .bind("receta de pasta carbonara")
            .execute(&pool)
            .await?;

        // FTS trigger should fire and index it
        let provider =
            crate::embeddings::create_provider(&crate::embeddings::EmbeddingConfig::default());
        let service = SearchService::new(provider);

        let results = service
            .search(&pool, "pasta", SearchType::Messages, 10)
            .await?;
        assert!(!results.is_empty());
        assert_eq!(results[0].id, "m1");
        assert_eq!(results[0].source, "message");
        Ok(())
    }
}
