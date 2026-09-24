use axum::extract::State;
use axum::Json;

use crate::errors::AppError;
use crate::models::*;
use crate::AppState;

pub async fn get_profile(State(state): State<AppState>) -> Result<Json<Profile>, AppError> {
    let db = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let profile = crate::db::repos::profiles::ProfilesRepo::get_or_create(&db)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(profile))
}

pub async fn update_profile(
    State(state): State<AppState>,
    Json(body): Json<UpdateProfile>,
) -> Result<Json<Profile>, AppError> {
    let db = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let profile = crate::db::repos::profiles::ProfilesRepo::update(
        &db,
        body.name.as_deref(),
        body.avatar_url.as_deref(),
        body.preferences.as_ref(),
    )
    .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(profile))
}
