use axum::extract::State;
use axum::Json;

use crate::errors::AppError;
use crate::models::*;
use crate::AppState;

pub async fn get_profile(State(state): State<AppState>) -> Result<Json<Profile>, AppError> {
    let profile = crate::db::repos::profiles::ProfilesRepo::get_or_create(&state.db).await?;
    Ok(Json(profile))
}

pub async fn update_profile(
    State(state): State<AppState>,
    Json(body): Json<UpdateProfile>,
) -> Result<Json<Profile>, AppError> {
    let profile = crate::db::repos::profiles::ProfilesRepo::update(
        &state.db,
        body.name.as_deref(),
        body.avatar_url.as_deref(),
        body.preferences.as_ref(),
    )
    .await?;
    Ok(Json(profile))
}
