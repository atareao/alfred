use axum::extract::{Path, State};
use axum::Json;

use crate::errors::AppError;
use crate::models::Tool;
use crate::AppState;

pub async fn list_tools(State(state): State<AppState>) -> Result<Json<Vec<Tool>>, AppError> {
    let tools = crate::db::repos::tools::ToolsRepo::list(&state.db).await?;
    Ok(Json(tools))
}

pub async fn toggle_tool(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Tool>, AppError> {
    let tool = crate::db::repos::tools::ToolsRepo::toggle_enabled(&state.db, &id).await?;
    tool.ok_or_else(|| AppError::NotFound(format!("Tool {} not found", id)))
        .map(Json)
}
