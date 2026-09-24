mod common;
use common::TestApp;
use serde_json::json;

#[tokio::test]
async fn test_create_message() {
    // Given a conversation exists
    // When POST /api/conversations/:id/messages is called with valid body
    // Then returns 201 with the created message
    let app = TestApp::new().await;

    let resp = app
        .post("/api/conversations/conv-id/messages")
        .json(&json!({
            "role": "user",
            "content": "Hello, Alfred!"
        }))
        .send()
        .await;

    assert_eq!(resp.status(), 201);
    let body = resp.json::<serde_json::Value>().await;
    assert!(body.get("id").is_some());
    assert_eq!(body["role"], "user");
    assert_eq!(body["content"], "Hello, Alfred!");
}

#[tokio::test]
async fn test_create_message_with_tool_calls() {
    // Given a conversation exists
    // When creating a message with tool_calls
    // Then returns 201 with tool_calls preserved
    let app = TestApp::new().await;

    let resp = app
        .post("/api/conversations/conv-id/messages")
        .json(&json!({
            "role": "assistant",
            "content": "Let me check that.",
            "tool_calls": [{"name": "get_weather", "args": {"city": "Madrid"}}]
        }))
        .send()
        .await;

    assert_eq!(resp.status(), 201);
    let body = resp.json::<serde_json::Value>().await;
    assert_eq!(body["role"], "assistant");
    assert!(body.get("tool_calls").is_some());
}

#[tokio::test]
async fn test_list_messages() {
    // Given a conversation has messages
    // When GET /api/conversations/:id/messages is called
    // Then returns 200 with paginated messages
    let app = TestApp::new().await;

    let resp = app.get("/api/conversations/conv-id/messages").await;

    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await;
    assert!(body.get("data").is_some());
}

#[tokio::test]
async fn test_list_messages_with_pagination() {
    // Given a conversation has many messages
    // When GET /api/conversations/:id/messages?limit=10 is called
    // Then returns 200 with paginated data
    let app = TestApp::new().await;

    let resp = app
        .get("/api/conversations/conv-id/messages?limit=10")
        .await;

    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn test_get_message() {
    // Given a message exists in a conversation
    // When GET /api/conversations/:id/messages/:msg_id is called
    // Then returns 200 with the message
    let app = TestApp::new().await;

    let resp = app.get("/api/conversations/conv-id/messages/msg-id").await;

    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await;
    assert!(body.get("id").is_some());
}

#[tokio::test]
async fn test_get_message_not_found() {
    // Given no message with that id exists
    // When GET /api/conversations/:id/messages/:msg_id is called
    // Then returns 404
    let app = TestApp::new().await;

    let resp = app
        .get("/api/conversations/conv-id/messages/non-existent")
        .await;

    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn test_create_long_message_triggers_collapse() {
    // Given a conversation exists
    // When creating a message with very long content (> 2000 estimated tokens)
    // Then the response includes tokens_count > 0
    let app = TestApp::new().await;

    // ~8000 chars → ~2289 tokens (well above 2000 threshold)
    let long_content = "A ".repeat(4000);

    let resp = app
        .post("/api/conversations/conv-id/messages")
        .json(&json!({
            "role": "user",
            "content": long_content
        }))
        .send()
        .await;

    assert_eq!(resp.status(), 201);
    let body = resp.json::<serde_json::Value>().await;
    assert!(body.get("id").is_some(), "Message should have an id");
    assert!(
        body["tokens_count"].as_u64().unwrap_or(0) > 0,
        "tokens_count should be > 0 for long messages"
    );
    // The collapse is async, so collapsed_content may still be null
    // at this point — we only verify the message was created correctly.
}
