mod common;
use common::TestApp;
use serde_json::json;

#[tokio::test]
async fn test_create_memory() {
    // Given a valid POST to /api/memories
    // When the handler processes it
    // Then returns 201 with the created memory
    let app = TestApp::new().await;

    let resp = app
        .post("/api/memories")
        .json(&json!({
            "content": "Alfred remembers this."
        }))
        .send()
        .await;

    assert_eq!(resp.status(), 201);
    let body = resp.json::<serde_json::Value>().await;
    assert!(body.get("id").is_some());
    assert_eq!(body["content"], "Alfred remembers this.");
}

#[tokio::test]
async fn test_create_memory_default_category() {
    // Given a POST without category
    // When the handler processes it
    // Then returns 201
    let app = TestApp::new().await;

    let resp = app
        .post("/api/memories")
        .json(&json!({
            "content": "Default category memory."
        }))
        .send()
        .await;

    assert_eq!(resp.status(), 201);
}

#[tokio::test]
async fn test_list_memories() {
    // Given there are memories
    // When GET /api/memories is called
    // Then returns 200 with paginated response
    let app = TestApp::new().await;

    let resp = app.get("/api/memories").await;

    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await;
    assert!(body.get("data").is_some());
}

#[tokio::test]
async fn test_list_memories_with_pagination() {
    // Given there are many memories
    // When GET /api/memories?limit=5 is called
    // Then returns 200 with paginated data
    let app = TestApp::new().await;

    let resp = app.get("/api/memories?limit=5").await;

    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn test_delete_memory() {
    // Given a memory exists
    // When DELETE /api/memories/:id is called
    // Then returns 204
    let app = TestApp::new().await;

    let resp = app.delete("/api/memories/memory-id").await;

    assert_eq!(resp.status(), 204);
}

#[tokio::test]
async fn test_delete_memory_not_found() {
    // Given no memory with that id exists
    // When DELETE /api/memories/:id is called
    // Then returns 404
    let app = TestApp::new().await;

    let resp = app.delete("/api/memories/non-existent").await;

    assert_eq!(resp.status(), 404);
}
