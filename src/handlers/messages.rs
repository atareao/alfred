use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;

use crate::errors::AppError;
use crate::models::*;
use crate::AppState;

pub async fn list_messages(
    State(state): State<AppState>,
    Path(conv_id): Path<String>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<Message>>, AppError> {
    let db = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    // Verify conversation exists
    let _conv = crate::db::repos::conversations::ConversationsRepo::find_by_id(&db, &conv_id)
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Conversation {} not found", conv_id)))?;

    let limit = params.limit.unwrap_or(50);
    let (data, next_cursor) = crate::db::repos::messages::MessagesRepo::list_by_conversation(
        &db,
        &conv_id,
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

pub async fn create_message(
    State(state): State<AppState>,
    Path(conv_id): Path<String>,
    Json(body): Json<CreateMessage>,
) -> Result<(StatusCode, Json<Message>), AppError> {
    let db = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let msg = crate::db::repos::messages::MessagesRepo::create(
        &db,
        &conv_id,
        &body.role,
        &body.content,
        body.tool_calls.as_ref(),
        body.tool_results.as_ref(),
        2000,
        None,
    )
    .map_err(|e| match e {
        rusqlite::Error::InvalidParameterName(_) => {
            AppError::NotFound(format!("Conversation {} not found", conv_id))
        }
        _ => AppError::Internal(e.to_string()),
    })?;
    Ok((StatusCode::CREATED, Json(msg)))
}

pub async fn get_message(
    State(state): State<AppState>,
    Path((_conv_id, msg_id)): Path<(String, String)>,
) -> Result<Json<Message>, AppError> {
    let db = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let msg = crate::db::repos::messages::MessagesRepo::find_by_id(&db, &msg_id)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    msg.ok_or_else(|| AppError::NotFound(format!("Message {} not found", msg_id)))
        .map(Json)
}
