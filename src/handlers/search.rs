use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::errors::AppError;
use crate::models::PaginatedResponse;
use crate::AppState;

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
    All,
}

impl SearchType {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "message" | "messages" => Self::Messages,
            _ => Self::All,
        }
    }
}

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

    let _search_type = SearchType::from_str(params.search_type.as_deref().unwrap_or("all"));
    let limit = params.limit.unwrap_or(20);

    // FTS5 search (messages only)
    let fts_results = {
        let mut results = Vec::new();

        let msgs = crate::db::fts::search_messages_fts(&state.db, &params.q, limit * 2).await?;
        for (id, content, _) in msgs {
            results.push((id, content, "message".to_string()));
        }

        results
    };

    // Score results
    let k = 60.0_f64;
    let mut scored: Vec<(String, f64, String)> = Vec::new(); // (id, score, source)

    for (rank, (id, _, source)) in fts_results.iter().enumerate() {
        scored.push((id.clone(), 1.0 / (k + rank as f64), source.clone()));
    }

    // Sort by score descending
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit.clamp(1, 100) as usize);

    // Get content for results
    let mut results = Vec::new();
    for (id, score, source) in scored {
        let content = if source == "message" {
            crate::db::repos::messages::MessagesRepo::find_by_id(&state.db, &id)
                .await
                .ok()
                .flatten()
                .map(|m| m.content)
                .unwrap_or_default()
        } else {
            String::new()
        };
        let created_at = String::new();
        results.push(SearchResult {
            id,
            content,
            score,
            source,
            created_at,
        });
    }

    let total = results.len() as i64;
    Ok(Json(PaginatedResponse {
        data: results,
        next_cursor: None,
        total: Some(total),
    }))
}
