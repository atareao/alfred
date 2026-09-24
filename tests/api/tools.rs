mod common;
use common::TestApp;

use axum::http::StatusCode;

#[tokio::test]
async fn test_list_tools() {
    // Given tools exist in the database
    // When GET /api/tools is called
    // Then returns 200 with a list of tools
    let app = TestApp::new().await;

    let resp = app.get("/api/tools").await;

    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await;
    assert!(body.is_array());
}

#[tokio::test]
async fn test_toggle_tool() {
    // Given a tool exists with enabled=true
    // First, get the list of tools to find a real ID
    let app = TestApp::new().await;

    let list_resp = app.get("/api/tools").await;
    let tools = list_resp.json::<serde_json::Value>().await;
    let tool_id = tools[0]["id"].as_str().unwrap().to_string();
    assert!(tools[0]["enabled"].as_bool().unwrap());

    // When PUT /api/tools/:id/toggle is called
    // Then returns 200 with enabled=false
    let resp = app
        .put(&format!("/api/tools/{}/toggle", tool_id))
        .json(&serde_json::json!({}))
        .send()
        .await;

    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await;
    assert!(!body["enabled"].as_bool().unwrap());
}

#[tokio::test]
async fn test_toggle_tool_not_found() {
    // Given no tool with that id exists
    // When PUT /api/tools/:id/toggle is called
    // Then returns 404
    let app = TestApp::new().await;

    let resp = app
        .put("/api/tools/non-existent/toggle")
        .json(&serde_json::json!({}))
        .send()
        .await;

    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn test_list_tools_includes_all_domain_tools() {
    // Given the database is seeded with the F5b domain tools
    // When GET /api/tools is called
    // Then the response contains calendar, tasks, reminders, knowledge, contacts
    let app = TestApp::new().await;

    let resp = app.get("/api/tools").await;

    assert_eq!(resp.status(), StatusCode::OK);
    let tools = resp.json::<serde_json::Value>().await;
    let names: Vec<&str> = tools
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert!(names.contains(&"calendar"), "Expected calendar tool");
    assert!(names.contains(&"tasks"), "Expected tasks tool");
    assert!(names.contains(&"reminders"), "Expected reminders tool");
    assert!(names.contains(&"knowledge"), "Expected knowledge tool");
    assert!(names.contains(&"contacts"), "Expected contacts tool");
}

#[tokio::test]
async fn test_list_tools_includes_unified_search() {
    // Given the database is seeded with the F5b unified_search tool
    // When GET /api/tools is called
    // Then the response contains the unified_search tool
    let app = TestApp::new().await;

    let resp = app.get("/api/tools").await;

    assert_eq!(resp.status(), StatusCode::OK);
    let tools = resp.json::<serde_json::Value>().await;
    let names: Vec<&str> = tools
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert!(
        names.contains(&"unified_search"),
        "Expected unified_search tool"
    );
}

#[tokio::test]
async fn test_toggle_tool_enabled() {
    // Given the calendar tool exists in a known enabled state
    // When PUT /api/tools/:id/toggle is called with the calendar tool id
    // Then the response returns the tool with the enabled flag flipped
    let app = TestApp::new().await;

    // First, list tools to get the calendar tool's ID and current state
    let resp = app.get("/api/tools").await;
    assert_eq!(resp.status(), StatusCode::OK);
    let tools = resp.json::<serde_json::Value>().await;
    let calendar_tool = tools
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "calendar")
        .expect("calendar tool should be seeded");
    let tool_id = calendar_tool["id"].as_str().unwrap();
    let was_enabled = calendar_tool["enabled"].as_bool().unwrap();

    // Toggle
    let resp = app
        .put(&format!("/api/tools/{}/toggle", tool_id))
        .json(&serde_json::json!({}))
        .send()
        .await;

    assert_eq!(resp.status(), StatusCode::OK);
    let toggled = resp.json::<serde_json::Value>().await;
    assert_eq!(toggled["enabled"].as_bool().unwrap(), !was_enabled);
}

#[tokio::test]
async fn test_list_tools_includes_new_f5c_tools() {
    // Given the database is seeded with the F5c domain tools
    // When GET /api/tools is called
    // Then the response contains weather, geo, meals, habits
    let app = TestApp::new().await;

    let resp = app.get("/api/tools").await;

    assert_eq!(resp.status(), StatusCode::OK);
    let tools = resp.json::<serde_json::Value>().await;
    let names: Vec<&str> = tools
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert!(names.contains(&"weather"), "Expected weather tool");
    assert!(names.contains(&"geo"), "Expected geo tool");
    assert!(names.contains(&"meals"), "Expected meals tool");
    assert!(names.contains(&"habits"), "Expected habits tool");
}

#[tokio::test]
async fn test_toggle_nonexistent_tool_returns_error() {
    // Given no tool with that id exists
    // When PUT /api/tools/:id/toggle is called with a nonexistent id
    // Then returns 404 with an error body
    let app = TestApp::new().await;

    let resp = app
        .put("/api/tools/nonexistent-id/toggle")
        .json(&serde_json::json!({}))
        .send()
        .await;

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body = resp.json::<serde_json::Value>().await;
    assert!(body["error"].is_string(), "Expected an error message");
}
