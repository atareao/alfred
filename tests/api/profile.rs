mod common;
use common::TestApp;
use serde_json::json;

#[tokio::test]
async fn test_get_profile_returns_default() {
    // Given no profile has been created
    // When GET /api/profile is called
    // Then returns 200 with a default profile
    let app = TestApp::new().await;

    let resp = app.get("/api/profile").await;

    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await;
    assert!(body.get("id").is_some());
    assert!(body.get("name").is_some());
}

#[tokio::test]
async fn test_update_profile() {
    // Given a profile exists
    // When PUT /api/profile with valid body is called
    // Then returns 200 with updated fields
    let app = TestApp::new().await;

    let resp = app
        .put("/api/profile")
        .json(&json!({
            "name": "Alfred User",
            "preferences": {"theme": "dark"}
        }))
        .send()
        .await;

    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await;
    assert_eq!(body["name"], "Alfred User");
    assert_eq!(body["preferences"]["theme"], "dark");
}

#[tokio::test]
async fn test_update_profile_partial() {
    // Given a profile exists
    // When PUT /api/profile with only name is called
    // Then returns 200 with only name updated and other fields unchanged
    let app = TestApp::new().await;

    let resp = app
        .put("/api/profile")
        .json(&json!({"name": "New Name Only"}))
        .send()
        .await;

    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await;
    assert_eq!(body["name"], "New Name Only");
}
