//! Full round-trip integration test for `GET /api/stats/llm/last-call`.
//!
//! This complements the handler-level tests in `src/handlers/stats.rs`
//! (`test_last_call_handler_*` / `test_last_call_via_http_router`) by exercising
//! the public crate surface end-to-end. `test_last_call_via_http_router` already
//! drives the real router through `crate::app_with_state`, so the genuine
//! differential of living under `tests/` is that this file proves `AppState`,
//! `app_with_state` and `models::stats::LastApiCall` are *publicly exportable*:
//! if any of them were made private this file would stop compiling while the
//! unit test would keep building. It also pins the full JSON shape and the
//! deterministic `created_at` (a fixed literal, never `Utc::now()`).
//!
//! It writes a `LastApiCall` into the shared state, serves it through the real
//! router (`app_with_state`), and then verifies the serialized JSON shape of
//! **all 14 fields**. Some of these the unit tests only check as Rust values,
//! never as rendered JSON — e.g. `test_last_call_handler_with_data` asserts
//! `data.cost == 0.01` but nothing in `src/handlers/stats.rs` verified how
//! `cost` serializes. Of the fields whose JSON the unit tests never inspect,
//! only `duration_ms` (`Option<i64>`), `error_message` (`Option<String>`) and
//! `tool_calls` (`Option<String>`) are `Option`; `prompt_tokens`,
//! `completion_tokens`, `cached_tokens` and `reasoning_tokens` are integers
//! (`u32`), `cost` is `f64` and `created_at` is a `String`. The handler returns
//! `Json<Option<LastApiCall>>` directly, so every field is expected to be
//! present with `None` rendered as JSON `null` (the model derives `Serialize`
//! with no `rename` / `skip_serializing_if` / `flatten` attributes).

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;

/// Verify the full pipeline: save data in state → handler serves it via HTTP.
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
        created_at: "2026-09-27T10:00:00Z".into(),
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

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    let parsed: Value = serde_json::from_str(&body_str).unwrap();

    // 4. Verify the full serialized shape (round-trip of all 14 fields)
    let obj = parsed
        .as_object()
        .expect("last-call response must be a JSON object");

    // The model has no `skip_serializing_if`, so every field must be present.
    //
    // This count is a deliberate contract test of the response shape, not an
    // incidental detail: it is meant to act as a change-detector. Adding a field
    // to `LastApiCall` will turn this assertion red on purpose, and the correct
    // reaction is to update this test together with the model — never to relax
    // or delete the count to silence it. Breaking here is the expected signal
    // that the public JSON contract changed.
    assert_eq!(
        obj.len(),
        14,
        "expected exactly the 14 LastApiCall fields, got {obj:?}"
    );

    // String fields
    assert_eq!(parsed["model"], "test-model-from-orchestrator");
    assert_eq!(parsed["request_body"], "{\"model\":\"test\"}");
    assert_eq!(parsed["response_body"], "{\"choices\":[]}");
    assert_eq!(parsed["status"], "success");
    assert_eq!(parsed["created_at"], "2026-09-27T10:00:00Z");

    // Numeric fields
    assert_eq!(parsed["prompt_tokens"], 10);
    assert_eq!(parsed["completion_tokens"], 20);
    assert_eq!(parsed["total_tokens"], 30);
    assert_eq!(parsed["cached_tokens"], 5);
    assert_eq!(parsed["reasoning_tokens"], 2);
    assert_eq!(parsed["cost"], 0.001);
    assert_eq!(parsed["duration_ms"], 100);

    // Option fields explicitly set to None must serialize as present-but-null
    // (no `skip_serializing_if` on the struct), which also proves the field is
    // part of the public JSON contract rather than silently omitted.
    assert!(
        obj.contains_key("error_message"),
        "error_message must be present in the response"
    );
    assert!(
        parsed["error_message"].is_null(),
        "error_message must be null when None"
    );
    assert!(
        obj.contains_key("tool_calls"),
        "tool_calls must be present in the response"
    );
    assert!(
        parsed["tool_calls"].is_null(),
        "tool_calls must be null when None"
    );
}
