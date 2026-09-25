use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;

/// RED phase test: asserts that GET /api/health returns 200 OK.
/// This will FAIL because the stub handler returns status = "starting".
#[tokio::test]
async fn test_health_status_200() {
    let app = alfred::app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

/// RED phase test: asserts that the JSON body contains status = "ok".
/// This will FAIL because the stub returns status = "starting".
#[tokio::test]
async fn test_health_body_status_ok() {
    let app = alfred::app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    let json: Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(json["status"], "ok");
}

/// RED phase test: asserts that the JSON body contains version = "0.1.0".
/// This will FAIL because the stub returns version = "0.0.0".
#[tokio::test]
async fn test_health_body_version() {
    let app = alfred::app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    let json: Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(json["version"], "0.1.0");
}

/// RED phase test: asserts that the JSON body contains db = "connected".
/// This will FAIL because the stub returns db = "disconnected".
#[tokio::test]
async fn test_health_body_db_connected() {
    let app = alfred::app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    let json: Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(json["db"], "connected");
}
