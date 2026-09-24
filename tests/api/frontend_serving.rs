use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

/// RED phase test: asserts that GET / returns index.html.
/// This will FAIL because the router doesn't have a ServeDir fallback yet.
#[tokio::test]
async fn test_root_returns_index_html() {
    let app = alfred::app();

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        content_type.contains("text/html"),
        "Expected text/html, got: {content_type}"
    );
}

/// RED phase test: asserts that GET /some/spa/route returns index.html (SPA fallback).
/// This will FAIL because the router doesn't have a ServeFile fallback.
#[tokio::test]
async fn test_spa_fallback_returns_index_html() {
    let app = alfred::app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/some/spa/route")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        content_type.contains("text/html"),
        "Expected text/html for SPA fallback, got: {content_type}"
    );
}

/// RED phase test: asserts that GET /api/health still returns JSON even with
/// the static file fallback in place.
#[tokio::test]
async fn test_api_health_unaffected_by_fallback() {
    let app = alfred::app();

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

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        content_type.contains("json"),
        "Expected application/json for API, got: {content_type}"
    );
}

/// RED phase test: asserts that GET /static/index.html serves the actual file.
/// This will FAIL because the router doesn't have a ServeDir fallback.
#[tokio::test]
async fn test_static_file_is_served() {
    let app = alfred::app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/index.html")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&body_bytes);
    assert!(
        body_str.contains("Alfred"),
        "Expected index.html content to contain 'Alfred', got: {body_str}"
    );
}
