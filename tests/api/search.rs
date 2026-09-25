mod common;
use common::TestApp;
use serde_json::json;

#[tokio::test]
async fn test_search_empty_query_returns_400() {
    let app = TestApp::new().await;
    let resp = app.get("/api/search?q=").await;
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn test_search_with_messages() {
    let app = TestApp::new().await;

    // Create a message directly
    app.post("/api/messages")
        .json(&json!({"role": "user", "content": "receta de pasta carbonara"}))
        .send()
        .await;

    // Search should find it via FTS5
    let resp = app.get("/api/search?q=pasta&type=message").await;
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await;
    assert!(body["data"].is_array());
}

#[tokio::test]
async fn test_search_with_memories() {
    let app = TestApp::new().await;

    // Create a memory
    app.post("/api/memories")
        .json(&json!({
            "profile_id": "profile-id",
            "content": "A Alfred le gusta el café",
            "category": "fact"
        }))
        .send()
        .await;

    let resp = app.get("/api/search?q=café&type=memory").await;
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn test_search_all_types() {
    let app = TestApp::new().await;
    let resp = app.get("/api/search?q=test&type=all").await;
    assert_eq!(resp.status(), 200);
}
