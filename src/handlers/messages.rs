use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;

use crate::errors::AppError;
use crate::models::*;
use crate::AppState;

pub async fn list_messages(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<Message>>, AppError> {
    let limit = if let Some(l) = params.limit {
        l
    } else {
        crate::db::repos::settings::SettingsRepo::get(&state.db, "message_page_size")
            .await
            .ok()
            .flatten()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(50)
    };
    let (data, next_cursor) = crate::db::repos::messages::MessagesRepo::list_all(
        &state.db,
        limit,
        params.cursor.as_deref(),
    )
    .await?;
    Ok(Json(PaginatedResponse {
        data,
        next_cursor,
        total: None,
    }))
}

pub async fn create_message(
    State(state): State<AppState>,
    Json(body): Json<CreateMessage>,
) -> Result<(StatusCode, Json<Message>), AppError> {
    let collapse_callback = state.collapse_tx.clone().map(|tx| {
        Box::new(move |msg_id: String| {
            let _ = tx.try_send(msg_id);
        }) as Box<dyn Fn(String) + Send>
    });

    let msg = crate::db::repos::messages::MessagesRepo::create(
        &state.db,
        &body.role,
        &body.content,
        body.tool_calls.as_ref(),
        body.tool_results.as_ref(),
        2000,
        collapse_callback,
    )
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok((StatusCode::CREATED, Json(msg)))
}

pub async fn get_message(
    State(state): State<AppState>,
    Path(msg_id): Path<String>,
) -> Result<Json<Message>, AppError> {
    let msg = crate::db::repos::messages::MessagesRepo::find_by_id(&state.db, &msg_id).await?;
    msg.ok_or_else(|| AppError::NotFound(format!("Message {} not found", msg_id)))
        .map(Json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tokio::sync::mpsc;
    use tower::ServiceExt;

    /// Given message_page_size = 25 in settings
    /// When GET /api/messages (without limit param)
    /// Then returns 25 messages (not the hardcoded 50)
    #[tokio::test]
    async fn test_list_messages_uses_setting_default() -> Result<(), Box<dyn std::error::Error>> {
        // ── Setup: in-memory DB with migrations ──────────────────────────
        let state = crate::AppState::new_in_memory_empty().await;

        // Insert 60 messages directly (no conversation needed)
        for i in 0..60 {
            crate::db::repos::messages::MessagesRepo::create(
                &state.db,
                "user",
                &format!("Message {}", i),
                None,
                None,
                2000,
                None,
            )
            .await?;
        }
        // Set message_page_size to 25
        crate::db::repos::settings::SettingsRepo::set(&state.db, "message_page_size", "25").await?;

        // ── Action: call handler via the router ──────────────────────────
        let app = crate::app_with_state(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/messages")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await?;

        // ── Assert: should return 25 (from setting), not 50 ──────────────
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await?;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let data = json["data"].as_array().unwrap();
        assert_eq!(
            data.len(),
            25,
            "Should return 25 messages (from message_page_size setting), not the hardcoded 50"
        );
        Ok(())
    }

    /// Given an AppState and a collapse channel,
    /// when a POST /api/messages with a long message (8000 chars) is sent,
    /// then the message_id SHOULD arrive via the collapse channel.
    #[tokio::test]
    async fn test_create_message_triggers_collapse_channel(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // ── Setup: in-memory DB with migrations ──────────────────────────
        let mut state = crate::AppState::new_in_memory_empty().await;

        // Create a collapse channel that SHOULD receive the message_id
        let (collapse_tx, mut collapse_rx) = mpsc::channel::<String>(16);
        state.collapse_tx = Some(collapse_tx);

        // ── Action: POST a long message (2000+ words for ~2660 tokens) ──────
        let app = crate::app_with_state(state);
        let long_content = "x ".repeat(2000);
        let body = serde_json::json!({
            "role": "user",
            "content": long_content,
        });

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/messages")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&body).unwrap()))
                    .unwrap(),
            )
            .await?;

        assert_eq!(resp.status(), StatusCode::CREATED);

        // ── Assert: the collapse channel should receive the message_id ─────
        let received =
            tokio::time::timeout(std::time::Duration::from_millis(500), collapse_rx.recv()).await;

        match received {
            Ok(Some(msg_id)) => {
                assert!(!msg_id.is_empty(), "message_id should not be empty");
            }
            _ => {
                panic!("Should have received message_id via collapse channel for a long message (8000 chars)");
            }
        }
        Ok(())
    }
}
