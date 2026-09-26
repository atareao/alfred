use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::db::repos::profiles::ProfilesRepo;
use crate::db::repos::tasks::TasksRepo;
use crate::errors::AppError;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub content: String,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub project: Option<String>,
    pub due_date: Option<String>,
    pub scope: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TaskFilters {
    pub status: Option<String>,
    pub priority: Option<String>,
    pub project: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskRequest {
    pub content: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub project: Option<String>,
    pub due_date: Option<String>,
    pub scope: Option<String>,
}

pub async fn list_tasks(
    State(state): State<AppState>,
    Query(filters): Query<TaskFilters>,
) -> Result<Json<Vec<crate::db::repos::tasks::Task>>, AppError> {
    let profile = ProfilesRepo::get_or_create(&state.db).await?;
    let tasks = TasksRepo::list(
        &state.db,
        &profile.id,
        filters.status.as_deref(),
        filters.priority.as_deref(),
        filters.project.as_deref(),
        None, // scope not exposed as query param in this API
    )
    .await?;
    Ok(Json(tasks))
}

pub async fn create_task(
    State(state): State<AppState>,
    Json(payload): Json<CreateTaskRequest>,
) -> Result<(StatusCode, Json<crate::db::repos::tasks::Task>), AppError> {
    let profile = ProfilesRepo::get_or_create(&state.db).await?;
    let now = chrono::Utc::now().to_rfc3339();
    let task = crate::db::repos::tasks::Task {
        id: Uuid::new_v4().to_string(),
        profile_id: profile.id,
        content: payload.content,
        status: payload.status.unwrap_or_else(|| "inbox".to_string()),
        priority: payload.priority.unwrap_or_else(|| "medium".to_string()),
        project: payload.project,
        due_date: payload.due_date,
        scope: payload.scope.unwrap_or_else(|| "shared".to_string()),
        created_at: now.clone(),
        updated_at: now,
    };
    TasksRepo::create(&state.db, &task).await?;
    Ok((StatusCode::CREATED, Json(task)))
}

pub async fn update_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateTaskRequest>,
) -> Result<Json<crate::db::repos::tasks::Task>, AppError> {
    TasksRepo::find_by_id(&state.db, &id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Task {} not found", id)))?;

    TasksRepo::update(
        &state.db,
        &id,
        payload.content.as_deref(),
        payload.status.as_deref(),
        payload.priority.as_deref(),
        payload.project.as_deref(),
        payload.due_date.as_deref(),
        payload.scope.as_deref(),
    )
    .await?;

    let updated = TasksRepo::find_by_id(&state.db, &id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Task {} not found after update", id)))?;
    Ok(Json(updated))
}

pub async fn delete_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let existing = TasksRepo::find_by_id(&state.db, &id).await?;
    if existing.is_none() {
        return Err(AppError::NotFound(format!("Task {} not found", id)));
    }
    TasksRepo::delete(&state.db, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
