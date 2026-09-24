mod common;
use common::TestApp;
use serde_json::json;

#[tokio::test]
async fn test_create_conversation_with_title() {
    // Given a valid POST request with a title
    // When the handler processes it
    // Then returns 201 with the created conversation
    let app = TestApp::new().await;

    let resp = app
        .post("/api/conversations")
        .json(&json!({"title": "Test Conversation"}))
        .send()
        .await;

    assert_eq!(resp.status(), 201);
    let body = resp.json::<serde_json::Value>().await;
    assert!(body.get("id").is_some());
    assert_eq!(body["title"], "Test Conversation");
}

#[tokio::test]
async fn test_create_conversation_without_title() {
    // Given a valid POST request without a title
    // When the handler processes it
    // Then returns 201 with an empty title
    let app = TestApp::new().await;

    let resp = app.post("/api/conversations").json(&json!({})).send().await;

    assert_eq!(resp.status(), 201);
    let body = resp.json::<serde_json::Value>().await;
    assert!(body.get("id").is_some());
    assert_eq!(body["title"], "");
}

#[tokio::test]
async fn test_list_conversations() {
    // Given there are conversations in the database
    // When GET /api/conversations is called
    // Then returns 200 with a paginated response
    let app = TestApp::new().await;

    let resp = app.get("/api/conversations").await;

    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await;
    assert!(body.get("data").is_some());
}

#[tokio::test]
async fn test_list_conversations_with_pagination() {
    // Given there are conversations
    // When GET /api/conversations?limit=10 is called
    // Then returns 200 with paginated data
    let app = TestApp::new().await;

    let resp = app.get("/api/conversations?limit=10").await;

    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn test_get_conversation() {
    // Given a conversation exists
    // When GET /api/conversations/:id is called
    // Then returns 200 with the conversation
    let app = TestApp::new().await;

    let resp = app.get("/api/conversations/some-id").await;

    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await;
    assert!(body.get("id").is_some());
}

#[tokio::test]
async fn test_get_conversation_not_found() {
    // Given no conversation with that id exists
    // When GET /api/conversations/:id is called
    // Then returns 404
    let app = TestApp::new().await;

    let resp = app.get("/api/conversations/non-existent-id").await;

    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn test_update_conversation() {
    // Given a conversation exists
    // When PUT /api/conversations/:id with title is called
    // Then returns 200 with updated conversation
    let app = TestApp::new().await;

    let resp = app
        .put("/api/conversations/some-id")
        .json(&json!({"title": "Updated Title"}))
        .send()
        .await;

    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await;
    assert_eq!(body["title"], "Updated Title");
}

#[tokio::test]
async fn test_delete_conversation() {
    // Given a conversation exists
    // When DELETE /api/conversations/:id is called
    // Then returns 204
    let app = TestApp::new().await;

    let resp = app.delete("/api/conversations/some-id").await;

    assert_eq!(resp.status(), 204);
}

#[tokio::test]
async fn test_conversation_lifecycle() {
    // Given the API is running
    // When creating, updating, getting, and deleting a conversation
    // Then all operations succeed
    let app = TestApp::new().await;

    // Create
    let resp = app
        .post("/api/conversations")
        .json(&json!({"title": "Lifecycle Test"}))
        .send()
        .await;
    assert_eq!(resp.status(), 201);
    let created = resp.json::<serde_json::Value>().await;
    let conv_id = created["id"].as_str().unwrap().to_string();

    // Get
    let resp = app.get(&format!("/api/conversations/{}", conv_id)).await;
    assert_eq!(resp.status(), 200);

    // Update
    let resp = app
        .put(&format!("/api/conversations/{}", conv_id))
        .json(&json!({"title": "Updated Lifecycle"}))
        .send()
        .await;
    assert_eq!(resp.status(), 200);

    // Delete
    let resp = app.delete(&format!("/api/conversations/{}", conv_id)).await;
    assert_eq!(resp.status(), 204);
}

#[tokio::test]
async fn test_get_main_conversation_creates_if_not_exists() {
    let app = TestApp::new_empty().await;

    let resp = app.get("/api/conversations/main").await;

    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await;
    assert_eq!(body["title"], "Alfred");
    assert!(body.get("id").is_some());
}

#[tokio::test]
async fn test_get_main_conversation_returns_existing() {
    let app = TestApp::new().await;

    // First call creates
    let resp1 = app.get("/api/conversations/main").await;
    let body1 = resp1.json::<serde_json::Value>().await;
    let id1 = body1["id"].as_str().unwrap().to_string();

    // Second call returns same
    let resp2 = app.get("/api/conversations/main").await;
    let body2 = resp2.json::<serde_json::Value>().await;
    assert_eq!(body2["id"], id1);
}
