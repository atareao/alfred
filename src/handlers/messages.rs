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

    let limit = if let Some(l) = params.limit {
        l
    } else {
        crate::db::repos::settings::SettingsRepo::get(&db, "message_page_size")
            .ok()
            .flatten()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(50)
    };
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    /// Given message_page_size = 25 in settings
    /// When GET /api/conversations/{id}/messages (without limit param)
    /// Then returns 25 messages (not the hardcoded 50)
    ///
    /// RED: The handler currently uses `params.limit.unwrap_or(50)` instead
    /// of reading from settings → this test WILL fail.
    #[tokio::test]
    async fn test_list_messages_uses_setting_default() {
        // ── Setup: in-memory DB with migrations ──────────────────────────
        let state = crate::AppState::new_in_memory_empty().await;

        // Create a conversation and insert 60 messages
        let conv_id = {
            let db = state.db.lock().unwrap();
            let conv =
                crate::db::repos::conversations::ConversationsRepo::create(&db, "Test").unwrap();
            let conv_id = conv.id.clone();
            for i in 0..60 {
                crate::db::repos::messages::MessagesRepo::create(
                    &db,
                    &conv_id,
                    "user",
                    &format!("Message {}", i),
                    None,
                    None,
                    2000,
                    None,
                )
                .unwrap();
            }
            // Set message_page_size to 25
            crate::db::repos::settings::SettingsRepo::set(&db, "message_page_size", "25").unwrap();
            conv_id
        };

        // ── Action: call handler via the router ──────────────────────────
        let app = crate::app_with_state(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(&format!("/api/conversations/{}/messages", conv_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // ── Assert: should return 25 (from setting), not 50 ──────────────
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let data = json["data"].as_array().unwrap();
        assert_eq!(
            data.len(),
            25,
            "Should return 25 messages (from message_page_size setting), not the hardcoded 50"
        );
    }
}
