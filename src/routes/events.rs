use axum::routing::{get, put};
use axum::Router;

use crate::handlers::events;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/events",
            get(events::list_events).post(events::create_event),
        )
        .route(
            "/api/events/:id",
            put(events::update_event).delete(events::delete_event),
        )
}
