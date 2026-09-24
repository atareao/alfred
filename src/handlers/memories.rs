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
    let db = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let limit = params.limit.unwrap_or(20);
    let offset = params.offset.unwrap_or(0);
    let (data, total) = crate::db::repos::memories::MemoriesRepo::list(&db, limit, offset)
        .map_err(|e| AppError::Internal(e.to_string()))?;
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
    let db = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let category = body.category.as_deref().unwrap_or("general");
    let source = body.source.as_deref().unwrap_or("manual");
    let memory = crate::db::repos::memories::MemoriesRepo::create(
        &db,
        &body.profile_id,
        &body.content,
        category,
        source,
    )
    .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok((StatusCode::CREATED, Json(memory)))
}

pub async fn delete_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let db = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let deleted = crate::db::repos::memories::MemoriesRepo::delete(&db, &id)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound(format!("Memory {} not found", id)))
    }
}
