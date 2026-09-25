use axum::extract::State;
use axum::Json;
use serde_json::Value;

use crate::errors::AppError;
use crate::AppState;

/// `GET /api/chat/init`
///
/// Returns both the most recent messages and current settings in a single
/// response, so the frontend can bootstrap the chat view in one round-trip.
pub async fn chat_init(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let limit = 50;
    let (messages, _) =
        crate::db::repos::messages::MessagesRepo::list_all(&state.db, limit, None).await?;
    let settings = crate::db::repos::settings::SettingsRepo::get_all(&state.db)
        .await
        .unwrap_or_default();
    Ok(Json(serde_json::json!({
        "messages": messages,
        "settings": settings,
    })))
}
