use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

use crate::AppState;

pub async fn get_settings(State(state): State<AppState>) -> Json<HashMap<String, String>> {
    let mut settings = crate::db::repos::settings::SettingsRepo::get_all(&state.db)
        .await
        .unwrap_or_default();

    // Include the default system prompt so the frontend can display it as a placeholder
    if let Some(orchestrator) = &state.orchestrator {
        settings.insert(
            "system_prompt_default".to_string(),
            orchestrator.config.system_prompt_template.clone(),
        );
    }

    Json(settings)
}

pub async fn update_settings(
    State(state): State<AppState>,
    Json(body): Json<HashMap<String, String>>,
) -> Result<Json<HashMap<String, String>>, (StatusCode, Json<JsonValue>)> {
    if body.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "No settings provided"})),
        ));
    }

    for (key, value) in &body {
        crate::db::repos::settings::SettingsRepo::set(&state.db, key, value)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": e.to_string()})),
                )
            })?;
    }

    let settings = crate::db::repos::settings::SettingsRepo::get_all(&state.db)
        .await
        .unwrap_or_default();
    Ok(Json(settings))
}
