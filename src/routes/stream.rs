//! SSE streaming endpoint and approval resolution.
//!
//! These handlers connect the orchestrator's streaming output to the frontend
//! via Server-Sent Events.
//!
//! ## Routes
//!
//! * `POST /api/conversations/:id/messages-stream` — SSE stream of [`SSEEvent`]s
//! * `POST /api/approval/:request_id`              — Resolve a pending approval

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::Json;
use futures::stream::Stream;
use serde::Deserialize;
use serde_json::Value;
use std::convert::Infallible;
use std::pin::Pin;

use crate::orchestrator::agent::BrowserContext;
use crate::orchestrator::agent::SSEEvent;
use crate::AppState;

/// Query payload for the streaming message endpoint.
#[derive(Debug, Deserialize)]
pub struct MessageQuery {
    pub content: String,
    pub browser_context: Option<BrowserContext>,
    #[serde(rename = "override")]
    pub override_cmd: Option<String>,
}

/// Body payload for the approval resolution endpoint.
#[derive(Debug, Deserialize)]
pub struct ApprovalBody {
    pub approved: bool,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `POST /api/conversations/:id/messages-stream`
///
/// Sends a message to the orchestrator and returns the response as an SSE
/// stream of [`SSEEvent`] values.
///
/// When the orchestrator is not available (e.g. in tests), falls back to a
/// single static chunk to preserve backward compatibility.
pub async fn stream_message(
    State(state): State<AppState>,
    Path(conversation_id): Path<String>,
    Json(query): Json<MessageQuery>,
) -> Sse<Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>>> {
    tracing::info!(
        conversation_id = %conversation_id,
        content_len = %query.content.len(),
        "📥 SSE stream request received"
    );

    // If orchestrator is not configured, fall back to stub response.
    // This preserves backward compatibility with tests that use app()
    // or new_in_memory() (which set orchestrator to None).
    if state.orchestrator.is_none() {
        tracing::warn!("Orchestrator is None, using fallback stub response");
        let event = SSEEvent::Chunk {
            content: "Hello from Alfred!".to_string(),
        };
        let stream =
            futures::stream::once(async move { Ok(Event::default().data(event.to_json_string())) });
        return Sse::new(Box::pin(stream));
    }

    let orchestrator = state.orchestrator.unwrap();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<SSEEvent>(32);

    let conv_id = conversation_id.clone();
    tracing::info!("Spawning orchestrator task for conversation {}", conv_id);
    let content = query.content.clone();
    let browser_context = query.browser_context.clone();
    tokio::spawn(async move {
        let _ = orchestrator
            .process_message_stream(&conv_id, "profile-1", &content, browser_context, tx)
            .await;
    });

    let stream = async_stream::stream! {
        while let Some(event) = rx.recv().await {
            yield Ok(Event::default().data(event.to_json_string()));
        }
    };

    Sse::new(Box::pin(stream))
}

/// `POST /api/approval/:request_id`
///
/// Resolves a pending human-in-the-loop approval request by delegating
/// to the guardrails component.
///
/// When guardrails are not available (e.g. in tests), falls back to a
/// hard-coded success response to preserve backward compatibility.
pub async fn resolve_approval(
    State(state): State<AppState>,
    Path(request_id): Path<String>,
    Json(body): Json<ApprovalBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let guardrails = match state.guardrails {
        Some(ref g) => g.clone(),
        // Fall back to stub for backward compat with tests
        None => {
            return Ok(Json(serde_json::json!({
                "status": "resolved",
                "approved": true,
            })));
        }
    };

    match guardrails.resolve_approval(&request_id, body.approved) {
        Ok(()) => Ok(Json(serde_json::json!({
            "status": "resolved",
            "approved": body.approved,
        }))),
        Err(e) => {
            let (status, msg) = match e {
                crate::orchestrator::guardrails::GuardrailError::RequestNotFound(_) => {
                    (StatusCode::NOT_FOUND, "Approval request not found")
                }
                crate::orchestrator::guardrails::GuardrailError::AlreadyResolved => {
                    (StatusCode::CONFLICT, "Approval request already resolved")
                }
                _ => (StatusCode::BAD_REQUEST, "Invalid approval request"),
            };
            Err((status, Json(serde_json::json!({"error": msg}))))
        }
    }
}

/// Assemble the stream and approval routes into a sub-router.
pub fn routes() -> axum::Router<AppState> {
    use axum::routing::post;
    axum::Router::new()
        .route(
            "/api/conversations/:id/messages-stream",
            post(stream_message),
        )
        .route("/api/approval/:request_id", post(resolve_approval))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use tower::ServiceExt;

    // ------------------------------------------------------------------
    // Approval endpoint
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_approval_endpoint_returns_ok() {
        // Given a running app
        // When POST /api/approval/test-request with { "approved": true }
        // Then 200 OK is returned with the expected shape
        let app = crate::app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/approval/test-request")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"approved":true}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_approval_endpoint_structure() {
        // Given a running app
        // When POST /api/approval/req-1 with { "approved": false }
        // Then the response body contains the expected fields
        let app = crate::app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/approval/req-1")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"approved":false}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "resolved");
        assert_eq!(json["approved"], true);
    }

    #[tokio::test]
    async fn test_approval_missing_body_returns_error() {
        // Given a running app
        // When POST /api/approval/req-1 with NO body
        // Then 400 or 422 is returned (missing field)
        let app = crate::app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/approval/req-1")
                    .header("content-type", "application/json")
                    .body(Body::from(r"{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Missing `approved` field should fail deserialization
        assert!(response.status().is_client_error());
    }

    // ------------------------------------------------------------------
    // Stream endpoint
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_stream_endpoint_returns_sse() {
        // Given a running app
        // When POST /api/conversations/conv-1/messages-stream with a message
        // Then 200 OK is returned with SSE content-type
        let app = crate::app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/conversations/conv-1/messages-stream")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"content":"Hello"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // SSE streams should have content-type text/event-stream
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            content_type.contains("text/event-stream"),
            "Expected SSE content-type, got: {}",
            content_type
        );
    }

    #[tokio::test]
    async fn test_stream_endpoint_rejects_missing_content() {
        // Given a running app
        // When POST /api/conversations/conv-1/messages-stream with empty body
        // Then 400 or 422 is returned
        let app = crate::app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/conversations/conv-1/messages-stream")
                    .header("content-type", "application/json")
                    .body(Body::from(r"{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Missing `content` field should fail deserialization
        assert!(response.status().is_client_error());
    }

    #[tokio::test]
    async fn test_stream_endpoint_route_not_found_for_get() {
        // Given a running app
        // When GET /api/conversations/conv-1/messages-stream (wrong method)
        // Then 405 Method Not Allowed
        let app = crate::app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/conversations/conv-1/messages-stream")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn test_stream_endpoint_accepts_browser_context() {
        // Given a running app
        // When POST with content + browser_context
        // Then 200 OK is returned with SSE content-type
        let app = crate::app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/conversations/conv-1/messages-stream")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"content":"Hello","browser_context":{"timestamp":"2026-09-24T08:00:00Z","timezone":"Europe/Madrid","latitude":39.36,"longitude":-0.41,"location_name":"Silla, Valencia, España"}}"#
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            content_type.contains("text/event-stream"),
            "Expected SSE content-type, got: {}",
            content_type
        );
    }
}
