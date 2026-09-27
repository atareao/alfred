// ⚠️ DEPRECATED — These handlers will be rewritten in Phase 6 to use MemoryRepo.
// For now, they delegate to MemoriesRepo (which itself delegates to MemoryRepo).

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;

use crate::errors::AppError;
use crate::models::*;
use crate::AppState;

pub async fn list_memories(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<Memory>>, AppError> {
    let limit = params.limit.unwrap_or(20);
    let offset = params.offset.unwrap_or(0);
    let (data, total) =
        crate::db::repos::memory::MemoryRepo::list(&state.db, limit, offset).await?;
    Ok(Json(PaginatedResponse {
        data,
        next_cursor: None,
        total: Some(total),
    }))
}

pub async fn create_memory(
    State(state): State<AppState>,
    Json(body): Json<CreateMemory>,
) -> Result<(StatusCode, Json<Memory>), AppError> {
    let content = &body.content;
    let tokens_count = body.tokens_count.unwrap_or(0);
    let metadata = body.metadata.unwrap_or(serde_json::json!({}));
    let memory =
        crate::db::repos::memory::MemoryRepo::create(&state.db, content, tokens_count, &metadata)
            .await?;
    Ok((StatusCode::CREATED, Json(memory)))
}

pub async fn delete_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let deleted = crate::db::repos::memory::MemoryRepo::delete(&state.db, &id).await?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound(format!("Memory {} not found", id)))
    }
}
