use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;

use crate::errors::AppError;
use crate::models::PaginatedResponse;
use crate::services::search_service::{SearchResult, SearchType};
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: String,
    #[serde(rename = "type")]
    pub search_type: Option<String>,
    pub limit: Option<i64>,
}

pub async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<PaginatedResponse<SearchResult>>, AppError> {
    if params.q.trim().is_empty() {
        return Err(AppError::BadRequest(
            "Query parameter 'q' is required".to_string(),
        ));
    }

    let search_type = SearchType::from_str(params.search_type.as_deref().unwrap_or("all"));
    let limit = params.limit.unwrap_or(20);

    // Phase 1: FTS5 search (sync, inside lock)
    let fts_results = {
        let db = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let mut results = Vec::new();

        // Search messages
        if matches!(search_type, SearchType::Messages | SearchType::All) {
            if let Ok(msgs) = crate::db::fts::search_messages_fts(&db, &params.q, limit * 2) {
                for (id, content, _) in msgs {
                    results.push((id, content, "message".to_string()));
                }
            }
        }

        // Search memories
        if matches!(search_type, SearchType::Memories | SearchType::All) {
            if let Ok(mems) = crate::db::fts::search_memories_fts(&db, &params.q, limit * 2) {
                for (id, content, _) in mems {
                    results.push((id, content, "memory".to_string()));
                }
            }
        }

        results
    };

    // Phase 2: Vector search (async, outside lock)
    let config = crate::embeddings::EmbeddingConfig::default();
    let provider = crate::embeddings::create_provider(&config);
    let embedding = provider.embed(&params.q).await.ok();

    // Phase 3: Vector DB lookup (sync, inside lock again)
    let vector_scores: Vec<(String, f64, String)> = if let Some(ref emb) = embedding {
        let db = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let mut scores = Vec::new();

        if matches!(search_type, SearchType::Messages | SearchType::All) {
            if let Ok(msgs) = crate::db::vector::search_message_vectors(&db, emb, limit * 2) {
                for (id, score) in msgs {
                    scores.push((id, score, "message".to_string()));
                }
            }
        }

        if matches!(search_type, SearchType::Memories | SearchType::All) {
            if let Ok(mems) = crate::db::vector::search_memory_vectors(&db, emb, limit * 2) {
                for (id, score) in mems {
                    scores.push((id, score, "memory".to_string()));
                }
            }
        }

        scores
    } else {
        Vec::new()
    };

    // Phase 4: RRF Fusion
    let k = 60.0_f64;
    let mut scored: Vec<(String, f64, String)> = Vec::new(); // (id, score, source)

    for (rank, (id, _, source)) in fts_results.iter().enumerate() {
        scored.push((id.clone(), 1.0 / (k + rank as f64), source.clone()));
    }

    for (vid, vscore, source) in &vector_scores {
        if let Some(existing) = scored
            .iter_mut()
            .find(|(id, _, s)| id == vid && s == source)
        {
            existing.1 += 1.0 / (k + 1.0);
        } else {
            scored.push((vid.clone(), *vscore, source.clone()));
        }
    }

    // Sort by score descending
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit.clamp(1, 100) as usize);

    // Phase 5: Get content for results
    let results: Vec<SearchResult> = {
        let db = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        scored
            .into_iter()
            .map(|(id, score, source)| {
                let content = if source == "message" {
                    crate::db::repos::messages::MessagesRepo::find_by_id(&db, &id)
                        .ok()
                        .flatten()
                        .map(|m| m.content)
                        .unwrap_or_default()
                } else {
                    crate::db::repos::memories::MemoriesRepo::find_by_id(&db, &id)
                        .ok()
                        .flatten()
                        .map(|m| m.content)
                        .unwrap_or_default()
                };
                let created_at = String::new();
                SearchResult {
                    id,
                    content,
                    score,
                    source,
                    created_at,
                }
            })
            .collect()
    };

    let total = results.len() as i64;
    Ok(Json(PaginatedResponse {
        data: results,
        next_cursor: None,
        total: Some(total),
    }))
}
