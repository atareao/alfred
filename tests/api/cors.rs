use axum::{body::Body, http::Request};
use tower::ServiceExt;

/// RED phase test: asserts that CORS headers are present in the response
/// when an Origin header is sent.
/// This will FAIL because the CorsLayer has not been added yet.
#[tokio::test]
async fn test_cors_headers_present() {
    let app = alfred::app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/health")
                .header("Origin", "http://localhost:5173")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Verificar que al menos un header CORS está presente
    let headers = response.headers();
    assert!(
        headers.contains_key("access-control-allow-origin")
            || headers.contains_key("access-control-allow-methods")
            || headers.contains_key("access-control-allow-headers"),
        "Expected at least one CORS header in response"
    );
}
