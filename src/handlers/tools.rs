use axum::extract::{Path, State};
use axum::Json;

use crate::errors::AppError;
use crate::models::Tool;
use crate::AppState;

pub async fn list_tools(State(state): State<AppState>) -> Result<Json<Vec<Tool>>, AppError> {
    let db = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let tools = crate::db::repos::tools::ToolsRepo::list(&db)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(tools))
}

pub async fn toggle_tool(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Tool>, AppError> {
    let db = state
        .db
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let tool = crate::db::repos::tools::ToolsRepo::toggle_enabled(&db, &id)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    tool.ok_or_else(|| AppError::NotFound(format!("Tool {} not found", id)))
        .map(Json)
}
