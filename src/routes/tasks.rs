use axum::routing::{get, put};
use axum::Router;

use crate::handlers::tasks;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/tasks",
            get(tasks::list_tasks).post(tasks::create_task),
        )
        .route(
            "/api/tasks/:id",
            put(tasks::update_task).delete(tasks::delete_task),
        )
}
