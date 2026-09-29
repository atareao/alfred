use std::sync::{Arc, RwLock};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use serde_json::Value;

/// Verify the full pipeline: save data in state → handler serves it via HTTP
#[tokio::test]
async fn test_last_call_full_pipeline() {
    // 1. Create state
    let state = valet::AppState::new_in_memory().await;
    
    // 2. Save data (simulating what Orchestrator::save_last_call does)
    let last = valet::models::stats::LastApiCall {
        model: "test-model-from-orchestrator".into(),
        request_body: Some("{\"model\":\"test\"}".into()),
        response_body: Some("{\"choices\":[]}".into()),
        prompt_tokens: 10,
        completion_tokens: 20,
        total_tokens: 30,
        cached_tokens: 5,
        reasoning_tokens: 2,
        cost: 0.001,
        duration_ms: Some(100),
        status: "success".into(),
        error_message: None,
        tool_calls: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    *state.last_api_call.write().unwrap() = Some(last);
    
    // 3. Build app and issue HTTP request
    let app = valet::app_with_state(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/stats/llm/last-call")
                .header("Content-Type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    let parsed: Value = serde_json::from_str(&body_str).unwrap();
    
    // 4. Verify data
    assert!(!parsed.is_null(), "Response should not be null");
    assert_eq!(parsed["model"], "test-model-from-orchestrator");
    assert_eq!(parsed["total_tokens"], 30);
    assert_eq!(parsed["status"], "success");
    assert_eq!(parsed["request_body"], "{\"model\":\"test\"}");
    assert_eq!(parsed["response_body"], "{\"choices\":[]}");
    
    println!("✅ Full pipeline integration test PASSED!");
}
