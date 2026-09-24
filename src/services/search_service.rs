use rusqlite::Connection;
use serde::Serialize;
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
        conn: &Connection,
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
                fts::search_messages_fts(conn, query, actual_limit * 2)
                    .map_err(|e| format!("FTS5 search error: {}", e))?
                    .into_iter()
                    .map(|(id, content, _)| (id, content, "message".to_string()))
                    .collect::<Vec<_>>()
            }
            _ => Vec::new(),
        };

        let fts_memories = match &search_type {
            SearchType::Memories | SearchType::All => {
                fts::search_memories_fts(conn, query, actual_limit * 2)
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
                        vector::search_message_vectors(conn, &embedding, actual_limit * 2)
                            .unwrap_or_default()
                            .into_iter()
                            .map(|(id, score)| (id, score, "message".to_string()))
                            .collect::<Vec<_>>()
                    }
                    _ => Vec::new(),
                };
                let mem_vec = match &search_type {
                    SearchType::Memories | SearchType::All => {
                        vector::search_memory_vectors(conn, &embedding, actual_limit * 2)
                            .unwrap_or_default()
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
        let results: Vec<SearchResult> = score_map
            .into_iter()
            .map(|(id, score, source)| {
                let content = get_content_by_id(&fts_results, &fts_memories, &id);
                let created_at = get_created_at(conn, &id, &source);
                SearchResult {
                    id,
                    content,
                    score,
                    source,
                    created_at,
                }
            })
            .collect();

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
fn get_created_at(conn: &Connection, id: &str, source: &str) -> String {
    let table = if source == "message" {
        "messages"
    } else {
        "memories"
    };
    if let Ok(mut stmt) = conn.prepare(&format!("SELECT created_at FROM {} WHERE id = ?1", table)) {
        if let Ok(mut rows) = stmt.query(rusqlite::params![id]) {
            if let Ok(Some(row)) = rows.next() {
                if let Ok(date) = row.get::<_, String>(0) {
                    return date;
                }
            }
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::fts as fts_mod;
    use crate::db::schema;
    use rusqlite::Connection;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        schema::run_migrations(&conn).unwrap();
        fts_mod::create_fts_triggers(&conn).unwrap();
        conn
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

    #[test]
    fn test_search_empty_query_returns_error() {
        let conn = setup();
        let provider =
            crate::embeddings::create_provider(&crate::embeddings::EmbeddingConfig::default());
        let service = SearchService::new(provider);

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(service.search(&conn, "", SearchType::All, 10));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Query cannot be empty");
    }

    #[test]
    fn test_search_fts5_returns_results() {
        let conn = setup();
        // Insert a conversation first (FK constraint)
        conn.execute(
            "INSERT INTO conversations (id, title) VALUES (?1, ?2)",
            rusqlite::params!["c1", "Test"],
        )
        .unwrap();
        // Insert a message
        conn.execute(
            "INSERT INTO messages (id, conversation_id, role, content) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params!["m1", "c1", "user", "receta de pasta carbonara"],
        )
        .unwrap();

        // FTS trigger should fire and index it
        let provider =
            crate::embeddings::create_provider(&crate::embeddings::EmbeddingConfig::default());
        let service = SearchService::new(provider);

        let rt = tokio::runtime::Runtime::new().unwrap();
        let results = rt
            .block_on(service.search(&conn, "pasta", SearchType::Messages, 10))
            .unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].id, "m1");
        assert_eq!(results[0].source, "message");
    }
}
