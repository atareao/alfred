mod common;
use common::TestApp;

use axum::http::StatusCode;

#[tokio::test]
async fn test_list_tasks_empty() {
    // Given no tasks in the database
    // When GET /api/tasks is called
    // Then returns 200 with an empty array
    let app = TestApp::new_empty().await;

    let resp = app.get("/api/tasks").await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.json::<serde_json::Value>().await;
    assert!(body.is_array());
    assert!(body.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_create_task() {
    // Given a valid JSON payload with content
    // When POST /api/tasks is called
    // Then returns 201 with the created task (including id, created_at)
    let app = TestApp::new_empty().await;

    let resp = app
        .post("/api/tasks")
        .json(&serde_json::json!({
            "content": "Buy groceries"
        }))
        .send()
        .await;

    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = resp.json::<serde_json::Value>().await;
    assert!(body["id"].is_string(), "Expected id field");
    assert_eq!(body["content"], "Buy groceries");
    assert_eq!(body["status"], "inbox");
    assert_eq!(body["priority"], "medium");
    assert_eq!(body["scope"], "shared");
    assert!(body["created_at"].is_string(), "Expected created_at field");
}

#[tokio::test]
async fn test_create_and_list() {
    // Given a task has been created
    // When GET /api/tasks is called
    // Then returns 200 with an array containing that task
    let app = TestApp::new_empty().await;

    let create_resp = app
        .post("/api/tasks")
        .json(&serde_json::json!({
            "content": "Write documentation"
        }))
        .send()
        .await;

    assert_eq!(create_resp.status(), StatusCode::CREATED);

    let resp = app.get("/api/tasks").await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.json::<serde_json::Value>().await;
    assert!(body.is_array());
    assert!(!body.as_array().unwrap().is_empty());
    let contents: Vec<&str> = body
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| e["content"].as_str())
        .collect();
    assert!(
        contents.contains(&"Write documentation"),
        "Expected created task in list"
    );
}

#[tokio::test]
async fn test_create_task_with_all_fields() {
    // Given a payload with all optional fields
    // When POST /api/tasks is called
    // Then returns 201 with all fields set accordingly
    let app = TestApp::new_empty().await;

    let resp = app
        .post("/api/tasks")
        .json(&serde_json::json!({
            "content": "High priority task",
            "status": "todo",
            "priority": "high",
            "project": "Alpha",
            "due_date": "2026-10-01T00:00:00Z",
            "scope": "personal"
        }))
        .send()
        .await;

    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = resp.json::<serde_json::Value>().await;
    assert_eq!(body["content"], "High priority task");
    assert_eq!(body["status"], "todo");
    assert_eq!(body["priority"], "high");
    assert_eq!(body["project"], "Alpha");
    assert_eq!(body["due_date"], "2026-10-01T00:00:00Z");
    assert_eq!(body["scope"], "personal");
}

#[tokio::test]
async fn test_update_task() {
    // Given a task exists
    // When PUT /api/tasks/:id is called with new content
    // Then returns 200 with the updated task
    let app = TestApp::new_empty().await;

    // Create a task first
    let create_resp = app
        .post("/api/tasks")
        .json(&serde_json::json!({
            "content": "Original task"
        }))
        .send()
        .await;

    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let created = create_resp.json::<serde_json::Value>().await;
    let task_id = created["id"].as_str().unwrap().to_string();

    // Update the task
    let resp = app
        .put(&format!("/api/tasks/{}", task_id))
        .json(&serde_json::json!({
            "content": "Updated task",
            "priority": "high",
            "project": "Beta"
        }))
        .send()
        .await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.json::<serde_json::Value>().await;
    assert_eq!(body["content"], "Updated task");
    assert_eq!(body["priority"], "high");
    assert_eq!(body["project"], "Beta");
    assert_eq!(body["id"], task_id);
}

#[tokio::test]
async fn test_update_task_not_found() {
    // Given no task with that id exists
    // When PUT /api/tasks/nonexistent is called
    // Then returns 404
    let app = TestApp::new_empty().await;

    let resp = app
        .put("/api/tasks/nonexistent")
        .json(&serde_json::json!({
            "content": "X"
        }))
        .send()
        .await;

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_task() {
    // Given a task exists
    // When DELETE /api/tasks/:id is called
    // Then returns 204 No Content
    let app = TestApp::new_empty().await;

    // Create a task first
    let create_resp = app
        .post("/api/tasks")
        .json(&serde_json::json!({
            "content": "Task to Delete"
        }))
        .send()
        .await;

    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let created = create_resp.json::<serde_json::Value>().await;
    let task_id = created["id"].as_str().unwrap().to_string();

    // Delete the task
    let resp = app.delete(&format!("/api/tasks/{}", task_id)).await;

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_delete_task_not_found() {
    // Given no task with that id exists
    // When DELETE /api/tasks/nonexistent is called
    // Then returns 404 Not Found
    let app = TestApp::new_empty().await;

    let resp = app.delete("/api/tasks/nonexistent").await;

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_with_filters() {
    // Given tasks with different statuses and projects
    // When GET /api/tasks?status=inbox is called
    // Then only inbox tasks are returned
    let app = TestApp::new_empty().await;

    // Create two tasks with different statuses
    let create1 = app
        .post("/api/tasks")
        .json(&serde_json::json!({
            "content": "Inbox task",
            "status": "inbox"
        }))
        .send()
        .await;
    assert_eq!(create1.status(), StatusCode::CREATED);
    let _task1 = create1.json::<serde_json::Value>().await;

    let create2 = app
        .post("/api/tasks")
        .json(&serde_json::json!({
            "content": "Done task",
            "status": "done"
        }))
        .send()
        .await;
    assert_eq!(create2.status(), StatusCode::CREATED);
    let _task2 = create2.json::<serde_json::Value>().await;

    // Filter by status = inbox
    let resp = app.get("/api/tasks?status=inbox").await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.json::<serde_json::Value>().await;
    assert!(body.is_array());
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["status"], "inbox");
    assert_eq!(body[0]["content"], "Inbox task");

    // Filter by status = done
    let resp = app.get("/api/tasks?status=done").await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.json::<serde_json::Value>().await;
    assert!(body.is_array());
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["status"], "done");

    // Filter by project
    let create3 = app
        .post("/api/tasks")
        .json(&serde_json::json!({
            "content": "Project task",
            "project": "Alpha"
        }))
        .send()
        .await;
    assert_eq!(create3.status(), StatusCode::CREATED);

    let resp = app.get("/api/tasks?project=Alpha").await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.json::<serde_json::Value>().await;
    assert!(body.is_array());
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["project"], "Alpha");
}

#[tokio::test]
async fn test_create_task_missing_content() {
    // When POST /api/tasks is called with empty body `{}`
    // Then returns 422 Unprocessable Entity
    let app = TestApp::new_empty().await;

    let resp = app
        .post("/api/tasks")
        .json(&serde_json::json!({}))
        .send()
        .await;

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
