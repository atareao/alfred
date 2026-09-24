use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;

use crate::errors::AppError;
use crate::models::*;
use crate::AppState;

pub async fn get_main_conversation(
    State(state): State<AppState>,
) -> Result<Json<Conversation>, AppError> {
    let db = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Try to find existing main conversation (first one by created_at)
    let existing = crate::db::repos::conversations::ConversationsRepo::find_first(&db)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if let Some(conv) = existing {
        return Ok(Json(conv));
    }

    // Create new main conversation
    let conv = crate::db::repos::conversations::ConversationsRepo::create(&db, "Alfred")
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(conv))
}

pub async fn list_conversations(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<Conversation>>, AppError> {
    let db = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let limit = params.limit.unwrap_or(20);
    let (data, next_cursor) = crate::db::repos::conversations::ConversationsRepo::list(
        &db,
        limit,
        params.cursor.as_deref(),
    )
    .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(PaginatedResponse {
        data,
        next_cursor,
        total: None,
    }))
}

pub async fn create_conversation(
    State(state): State<AppState>,
    Json(body): Json<CreateConversation>,
) -> Result<(StatusCode, Json<Conversation>), AppError> {
    let db = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let title = body.title.unwrap_or_default();
    let conv = crate::db::repos::conversations::ConversationsRepo::create(&db, &title)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok((StatusCode::CREATED, Json(conv)))
}

pub async fn get_conversation(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Conversation>, AppError> {
    let db = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let conv = crate::db::repos::conversations::ConversationsRepo::find_by_id(&db, &id)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    conv.ok_or_else(|| AppError::NotFound(format!("Conversation {} not found", id)))
        .map(Json)
}

pub async fn update_conversation(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateConversation>,
) -> Result<Json<Conversation>, AppError> {
    let db = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let conv =
        crate::db::repos::conversations::ConversationsRepo::update(&db, &id, body.title.as_deref())
            .map_err(|e| AppError::Internal(e.to_string()))?;
    conv.ok_or_else(|| AppError::NotFound(format!("Conversation {} not found", id)))
        .map(Json)
}

pub async fn delete_conversation(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let db = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let deleted = crate::db::repos::conversations::ConversationsRepo::delete(&db, &id)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound(format!("Conversation {} not found", id)))
    }
}
