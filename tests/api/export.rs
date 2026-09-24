use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn test_export_returns_json() {
    let app = alfred::app();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/export")
                .header("Content-Type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let data: Value = serde_json::from_slice(&body).unwrap();
    // All core tables should be present in the export
    assert!(data.get("profiles").is_some(), "missing profiles");
    assert!(data.get("conversations").is_some(), "missing conversations");
    assert!(data.get("messages").is_some(), "missing messages");
    assert!(data.get("events").is_some(), "missing events");
    assert!(data.get("tasks").is_some(), "missing tasks");
    assert!(data.get("notes").is_some(), "missing notes");
    assert!(data.get("contacts").is_some(), "missing contacts");
    assert!(data.get("reminders").is_some(), "missing reminders");
    assert!(data.get("meal_plans").is_some(), "missing meal_plans");
    assert!(data.get("shopping_list").is_some(), "missing shopping_list");
    assert!(data.get("habits").is_some(), "missing habits");
    assert!(data.get("habit_logs").is_some(), "missing habit_logs");
    assert!(data.get("memories").is_some(), "missing memories");
    assert!(data.get("tools").is_some(), "missing tools");
}

#[tokio::test]
async fn test_export_contains_seeded_data() {
    let app = alfred::app();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/export")
                .header("Content-Type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let data: Value = serde_json::from_slice(&body).unwrap();

    // Seeded data should be present
    let profiles = data["profiles"].as_array().unwrap();
    assert!(!profiles.is_empty(), "expected at least one profile");
    assert_eq!(profiles[0]["name"], "Test User");
}
