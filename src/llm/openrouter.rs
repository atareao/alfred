use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use std::pin::Pin;
use std::time::Duration;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::Stream;

use super::provider::{
    ChatMessage, ChatRequest, ChatResponse, LLMError, LLMProvider, StreamEvent, TokenUsage,
    ToolCall,
};

/// Configuration for the OpenRouter LLM provider.
#[derive(Debug, Clone)]
pub struct OpenRouterConfig {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
    pub max_retries: u32,
    pub timeout_secs: u64,
}

/// Provider that routes requests through the OpenRouter API.
pub struct OpenRouterProvider {
    config: OpenRouterConfig,
    client: Client,
}

impl OpenRouterProvider {
    pub fn new(config: OpenRouterConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .unwrap_or_default();
        Self { config, client }
    }

    /// Parse an OpenRouter JSON response body into a `ChatResponse`.
    ///
    /// This method is extracted for testability — it can be tested without an HTTP server.
    ///
    /// It parses `tool_calls` from the response when present. Each tool call's
    /// `function.arguments` (a JSON string) is parsed into a `serde_json::Value`;
    /// if the string is not valid JSON, it falls back to a `Value::String`.
    fn parse_response(body: &Value) -> Result<ChatResponse, LLMError> {
        let message = &body["choices"][0]["message"];

        // `content` may be `null` when tool_calls are present → fall back to empty string.
        let content = message["content"].as_str().unwrap_or("").to_string();

        // Parse tool_calls if present.
        let tool_calls = message["tool_calls"].as_array().map(|calls| {
            calls
                .iter()
                .map(|call| {
                    let id = call["id"].as_str().unwrap_or("").to_string();
                    let name = call["function"]["name"].as_str().unwrap_or("").to_string();
                    // `function.arguments` is a JSON string. Parse it; if it is not
                    // valid JSON, fall back to the raw string as a `Value::String`.
                    let arguments = call["function"]["arguments"]
                        .as_str()
                        .map(|s| {
                            serde_json::from_str::<Value>(s)
                                .unwrap_or_else(|_| Value::String(s.to_string()))
                        })
                        .unwrap_or(Value::Null);
                    ToolCall {
                        id,
                        name,
                        arguments,
                    }
                })
                .collect()
        });

        let prompt_tokens = body["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32;
        let completion_tokens = body["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32;

        Ok(ChatResponse {
            message: ChatMessage {
                role: "assistant".into(),
                content,
                tool_calls,
                tool_result: None,
                tool_call_id: None,
            },
            usage: Some(super::provider::TokenUsage {
                prompt_tokens,
                completion_tokens,
            }),
        })
    }
}

#[async_trait]
impl LLMProvider for OpenRouterProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, LLMError> {
        let url = format!("{}/chat/completions", self.config.base_url);

        let mut body = serde_json::json!({
            "model": self.config.model,
            "messages": request.messages.iter().map(|m| {
                let mut msg = serde_json::json!({
                    "role": m.role,
                    "content": m.content,
                });
                if let Some(ref tool_call_id) = m.tool_call_id {
                    msg["tool_call_id"] = serde_json::json!(tool_call_id);
                }
                if let Some(ref tool_calls) = m.tool_calls {
                    msg["tool_calls"] = serde_json::json!(tool_calls.iter().map(|tc| {
                        serde_json::json!({
                            "id": tc.id,
                            "type": "function",
                            "function": {
                                "name": tc.name,
                                "arguments": tc.arguments.to_string(),
                            }
                        })
                    }).collect::<Vec<_>>());
                }
                msg
            }).collect::<Vec<_>>(),
            "stream": false,
        });

        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }
        if let Some(max_t) = request.max_tokens {
            body["max_tokens"] = serde_json::json!(max_t);
        }
        if let Some(tools) = &request.tools {
            body["tools"] = serde_json::json!(tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.parameters,
                        }
                    })
                })
                .collect::<Vec<_>>());
        }

        tracing::debug!(
            model = %self.config.model,
            message_count = %request.messages.len(),
            "Sending request to OpenRouter"
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await;

        match &response {
            Ok(resp) => tracing::debug!(status = %resp.status(), "OpenRouter response received"),
            Err(e) => tracing::error!(error = %e, "OpenRouter request failed"),
        }

        let response = response.map_err(|e| {
            if e.is_timeout() {
                LLMError::Timeout(e.to_string())
            } else if let Some(status) = e.status() {
                match status.as_u16() {
                    429 => LLMError::RateLimited { retry_after: 30 },
                    401 => LLMError::AuthError("Invalid API key".into()),
                    _ => LLMError::HttpError(format!("HTTP {}: {}", status, e)),
                }
            } else {
                LLMError::HttpError(e.to_string())
            }
        })?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            // Read the response body to get error details from the API
            let body_text = response
                .text()
                .await
                .map_err(|e| LLMError::HttpError(format!("Failed to read error body: {}", e)))?;
            let details = if body_text.is_empty() {
                format!("HTTP {}", status)
            } else if let Ok(body_json) = serde_json::from_str::<Value>(&body_text) {
                let msg = body_json["error"]["message"]
                    .as_str()
                    .or_else(|| body_json["error"].as_str())
                    .unwrap_or(&body_text);
                format!("HTTP {} — {}", status, msg)
            } else {
                format!("HTTP {} — {}", status, body_text.trim())
            };
            return match status {
                429 => Err(LLMError::RateLimited { retry_after: 30 }),
                401 => Err(LLMError::AuthError(format!("Invalid API key: {}", details))),
                _ => Err(LLMError::HttpError(details)),
            };
        }

        let response_body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| LLMError::HttpError(format!("Failed to parse response: {}", e)))?;

        let chat_response = Self::parse_response(&response_body)?;

        tracing::debug!(
            content_len = %chat_response.message.content.len(),
            tool_calls = ?chat_response.message.tool_calls.as_ref().map(|t| t.len()),
            prompt_tokens = %chat_response.usage.as_ref().map(|u| u.prompt_tokens).unwrap_or(0),
            completion_tokens = %chat_response.usage.as_ref().map(|u| u.completion_tokens).unwrap_or(0),
            "OpenRouter response parsed successfully"
        );

        Ok(chat_response)
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, LLMError>> + Send>>, LLMError> {
        let url = format!("{}/chat/completions", self.config.base_url);

        // Build request body with stream: true
        let mut body = serde_json::json!({
            "model": self.config.model,
            "messages": request.messages.iter().map(|m| {
                let mut msg = serde_json::json!({
                    "role": m.role,
                    "content": m.content,
                });
                if let Some(ref tool_call_id) = m.tool_call_id {
                    msg["tool_call_id"] = serde_json::json!(tool_call_id);
                }
                if let Some(ref tool_calls) = m.tool_calls {
                    msg["tool_calls"] = serde_json::json!(tool_calls.iter().map(|tc| {
                        serde_json::json!({
                            "id": tc.id,
                            "type": "function",
                            "function": {
                                "name": tc.name,
                                "arguments": tc.arguments.to_string(),
                            }
                        })
                    }).collect::<Vec<_>>());
                }
                msg
            }).collect::<Vec<_>>(),
            "stream": true,
        });

        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }
        if let Some(max_t) = request.max_tokens {
            body["max_tokens"] = serde_json::json!(max_t);
        }
        if let Some(tools) = &request.tools {
            body["tools"] = serde_json::json!(tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.parameters,
                        }
                    })
                })
                .collect::<Vec<_>>());
        }

        tracing::debug!(
            model = %self.config.model,
            message_count = %request.messages.len(),
            "Sending streaming request to OpenRouter"
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await;

        match &response {
            Ok(resp) => {
                tracing::debug!(status = %resp.status(), "OpenRouter streaming response received")
            }
            Err(e) => tracing::error!(error = %e, "OpenRouter streaming request failed"),
        }

        let mut response = response.map_err(|e| {
            if e.is_timeout() {
                LLMError::Timeout(e.to_string())
            } else if let Some(status) = e.status() {
                match status.as_u16() {
                    429 => LLMError::RateLimited { retry_after: 30 },
                    401 => LLMError::AuthError("Invalid API key".into()),
                    _ => LLMError::HttpError(format!("HTTP {}: {}", status, e)),
                }
            } else {
                LLMError::HttpError(e.to_string())
            }
        })?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body_text = response
                .text()
                .await
                .map_err(|e| LLMError::HttpError(format!("Failed to read error body: {}", e)))?;
            let details = if body_text.is_empty() {
                format!("HTTP {}", status)
            } else if let Ok(body_json) = serde_json::from_str::<Value>(&body_text) {
                let msg = body_json["error"]["message"]
                    .as_str()
                    .or_else(|| body_json["error"].as_str())
                    .unwrap_or(&body_text);
                format!("HTTP {} — {}", status, msg)
            } else {
                format!("HTTP {} — {}", status, body_text.trim())
            };
            return match status {
                429 => Err(LLMError::RateLimited { retry_after: 30 }),
                401 => Err(LLMError::AuthError(format!("Invalid API key: {}", details))),
                _ => Err(LLMError::HttpError(details)),
            };
        }

        let (tx, rx) = tokio::sync::mpsc::channel::<Result<StreamEvent, LLMError>>(64);

        tokio::spawn(async move {
            let mut buffer = String::new();
            let mut acc = StreamAccumulator::default();

            loop {
                match response.chunk().await {
                    Ok(Some(chunk)) => {
                        let chunk_str = String::from_utf8_lossy(&chunk);
                        buffer.push_str(&chunk_str);

                        // Process all complete lines in the buffer
                        while let Some(newline_pos) = buffer.find('\n') {
                            let line = buffer[..newline_pos].to_string();
                            buffer = buffer[newline_pos + 1..].to_string();

                            let trimmed = line.trim();
                            if trimmed.is_empty() {
                                continue;
                            }

                            match parse_sse_event(trimmed, &mut acc) {
                                Ok(Some(event)) => {
                                    if tx.send(Ok(event)).await.is_err() {
                                        return;
                                    }
                                }
                                Ok(None) => {
                                    // Ignorable line, continue
                                }
                                Err(e) => {
                                    if tx.send(Err(e)).await.is_err() {
                                        return;
                                    }
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        // Stream ended
                        break;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Err(LLMError::HttpError(format!(
                                "Stream read error: {}",
                                e
                            ))))
                            .await;
                        return;
                    }
                }
            }
        });

        let stream = ReceiverStream::new(rx);
        Ok(Box::pin(stream))
    }

    async fn embed(&self, input: &str) -> Result<Vec<f32>, LLMError> {
        let url = format!("{}/embeddings", self.config.base_url);
        let body = serde_json::json!({
            "model": "openai/text-embedding-3-small",
            "input": input,
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| LLMError::HttpError(e.to_string()))?;

        let response_body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| LLMError::HttpError(format!("Failed to parse: {}", e)))?;

        let embedding: Vec<f32> = response_body["data"][0]["embedding"]
            .as_array()
            .ok_or_else(|| LLMError::Internal("No embedding in response".into()))?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        Ok(embedding)
    }
}

// ---------------------------------------------------------------------------
// SSE streaming support — RED phase: stub only, will be implemented later.
// ---------------------------------------------------------------------------

/// Accumulates partial tool call state across multiple SSE chunks.
#[derive(Debug, Default, Clone)]
pub struct StreamAccumulator {
    pub partial_tool_calls: Vec<PartialToolCall>,
    pub usage: Option<TokenUsage>,
}

impl StreamAccumulator {
    /// Consume all partial tool calls and produce a Vec<ToolCall>.
    /// Each arguments string is parsed as JSON (falling back to Value::String).
    /// Returns None if no tool calls were accumulated.
    pub fn finalize(&mut self) -> Option<Vec<ToolCall>> {
        if self.partial_tool_calls.is_empty() {
            return None;
        }
        let calls: Vec<ToolCall> = self
            .partial_tool_calls
            .drain(..)
            .map(|p| {
                let parsed_args = serde_json::from_str::<Value>(&p.arguments)
                    .unwrap_or_else(|_| Value::String(p.arguments.clone()));
                ToolCall {
                    id: p.id,
                    name: p.name,
                    arguments: parsed_args,
                }
            })
            .collect();
        Some(calls)
    }
}

/// A tool call that is being built up incrementally from SSE delta chunks.
#[derive(Debug, Clone)]
pub struct PartialToolCall {
    pub index: usize,
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// Parse a single SSE line from an OpenRouter streaming response.
///
/// The line may optionally begin with `"data: "` (which is stripped before
/// parsing). Returns `Ok(None)` for empty/ignorable lines, and
/// `Ok(Some(StreamEvent))` for meaningful events.
///
/// **Note:** This is a **RED-phase stub** — it will be implemented in a later
/// phase. Currently it always panics with `unimplemented!()`.
pub fn parse_sse_event(
    line: &str,
    acc: &mut StreamAccumulator,
) -> Result<Option<StreamEvent>, LLMError> {
    // 1. Strip "data: " prefix
    let body_str = if let Some(stripped) = line.strip_prefix("data: ") {
        if stripped == "[DONE]" || stripped.is_empty() {
            return Ok(None);
        }
        stripped
    } else {
        // No "data: " prefix — this is not an SSE data line.
        // Could be "event:", ":" (SSE comment), or raw JSON (tests).
        // If it starts with '{' or '[', treat as raw JSON for test compatibility.
        let trimmed = line.trim();
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            trimmed
        } else {
            return Ok(None);
        }
    };

    // 2. Parse JSON
    let body: Value = match serde_json::from_str(body_str) {
        Ok(v) => v,
        Err(e) => {
            return Err(LLMError::HttpError(format!(
                "Failed to parse SSE line: {}",
                e
            )))
        }
    };

    // 3. Extract choices[0]
    let choice = match body.get("choices").and_then(|c| c.get(0)) {
        Some(v) if v.is_object() => v,
        _ => return Ok(None),
    };

    // 4. Extract delta
    let delta = &choice["delta"];

    // 5. Process tool_calls
    if let Some(tool_calls) = delta["tool_calls"].as_array() {
        for tc in tool_calls {
            let index = tc["index"].as_u64().unwrap_or(0) as usize;

            // Find existing partial tool call in the accumulator
            let existing_pos = acc.partial_tool_calls.iter().position(|p| p.index == index);

            match existing_pos {
                None => {
                    // New tool call — create a PartialToolCall in the accumulator
                    let id = tc["id"].as_str().unwrap_or("").to_string();
                    let name = tc["function"]["name"].as_str().unwrap_or("").to_string();
                    let args = tc["function"]["arguments"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();

                    acc.partial_tool_calls.push(PartialToolCall {
                        index,
                        id: id.clone(),
                        name: name.clone(),
                        arguments: args,
                    });

                    // If this is a "header" chunk (has id + name), emit a ToolCall event immediately
                    if !id.is_empty() && !name.is_empty() {
                        return Ok(Some(StreamEvent::ToolCall(ToolCall {
                            id,
                            name,
                            arguments: Value::Null,
                        })));
                    }
                }
                Some(pos) => {
                    // Existing tool call — concatenate arguments
                    if let Some(new_args) = tc["function"]["arguments"].as_str() {
                        if !new_args.is_empty() {
                            acc.partial_tool_calls[pos].arguments.push_str(new_args);
                        }
                    }
                }
            }
        }
    }

    // 6. Process content
    if let Some(content) = delta["content"].as_str() {
        if !content.is_empty() {
            return Ok(Some(StreamEvent::Chunk(content.to_string())));
        }
    }

    // 7. Process finish_reason (stop OR tool_calls)
    if let Some(finish_reason) = choice["finish_reason"].as_str() {
        if finish_reason == "stop" || finish_reason == "tool_calls" {
            // Extract usage from the body (may be None for tool_calls chunks)
            if let Some(usage) = body.get("usage") {
                let prompt_tokens = usage["prompt_tokens"].as_u64().unwrap_or(0) as u32;
                let completion_tokens = usage["completion_tokens"].as_u64().unwrap_or(0) as u32;
                acc.usage = Some(TokenUsage {
                    prompt_tokens,
                    completion_tokens,
                });
            }

            // Finalize any accumulated tool calls
            let tool_calls = acc.finalize();

            return Ok(Some(StreamEvent::Done(ChatResponse {
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: String::new(),
                    tool_calls,
                    tool_result: None,
                    tool_call_id: None,
                },
                usage: acc.usage.clone(),
            })));
        }
    }

    // 8. Default: ignore empty deltas, no-ops, etc.
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_response_body_with_tool_calls() -> Value {
        serde_json::json!({
            "choices": [{
                "message": {
                    "content": null,
                    "tool_calls": [
                        {
                            "id": "call_abc123",
                            "type": "function",
                            "function": {
                                "name": "geo",
                                "arguments": r#"{"operation": "geocode", "query": "Silla, Valencia"}"#
                            }
                        },
                        {
                            "id": "call_def456",
                            "type": "function",
                            "function": {
                                "name": "get_weather",
                                "arguments": r#"{"city": "Madrid", "units": "celsius"}"#
                            }
                        }
                    ]
                }
            }],
            "usage": {
                "prompt_tokens": 50,
                "completion_tokens": 30
            }
        })
    }

    fn make_response_body_no_tool_calls() -> Value {
        serde_json::json!({
            "choices": [{
                "message": {
                    "content": "Hello! I'm an AI assistant."
                }
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5
            }
        })
    }

    pub fn make_response_body_invalid_arguments() -> Value {
        serde_json::json!({
            "choices": [{
                "message": {
                    "content": null,
                    "tool_calls": [
                        {
                            "id": "call_bad_args",
                            "type": "function",
                            "function": {
                                "name": "bad_tool",
                                "arguments": "this is not valid json"
                            }
                        }
                    ]
                }
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5
            }
        })
    }

    #[test]
    fn test_openrouter_config_defaults() {
        let config = OpenRouterConfig {
            api_key: "test-key".into(),
            model: "anthropic/claude-sonnet-20241022".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            max_retries: 3,
            timeout_secs: 60,
        };
        assert_eq!(config.model, "anthropic/claude-sonnet-20241022");
    }

    #[test]
    fn test_openrouter_provider_creation() {
        let config = OpenRouterConfig {
            api_key: "test-key".into(),
            model: "test-model".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            max_retries: 3,
            timeout_secs: 60,
        };
        let provider = OpenRouterProvider::new(config);
        // Just verify it compiles and doesn't panic
        let _ = provider;
    }

    // ---------------------------------------------------------------------------
    // RED phase: These tests will FAIL because parse_response currently does NOT
    //            parse tool_calls (returns None).
    // ---------------------------------------------------------------------------

    #[test]
    fn test_openrouter_parses_tool_calls() {
        let body = make_response_body_with_tool_calls();
        let response = OpenRouterProvider::parse_response(&body).unwrap();

        let tool_calls = response.message.tool_calls.expect(
            "Expected tool_calls to be Some, but got None. BUG: parse_response ignores tool_calls.",
        );

        assert_eq!(tool_calls.len(), 2, "Expected 2 tool calls");

        // First tool call
        assert_eq!(tool_calls[0].id, "call_abc123");
        assert_eq!(tool_calls[0].name, "geo");
        assert_eq!(
            tool_calls[0].arguments,
            serde_json::json!({"operation": "geocode", "query": "Silla, Valencia"})
        );

        // Second tool call
        assert_eq!(tool_calls[1].id, "call_def456");
        assert_eq!(tool_calls[1].name, "get_weather");
        assert_eq!(
            tool_calls[1].arguments,
            serde_json::json!({"city": "Madrid", "units": "celsius"})
        );

        // Content should be empty string when LLM returns null
        assert_eq!(
            response.message.content, "",
            "Expected empty content when LLM returns tool_calls"
        );
    }

    #[test]
    fn test_openrouter_no_tool_calls() {
        let body = make_response_body_no_tool_calls();
        let response = OpenRouterProvider::parse_response(&body).unwrap();

        // No tool_calls in this response
        assert!(
            response.message.tool_calls.is_none(),
            "Expected no tool_calls in normal response"
        );

        // Content should contain the response text
        assert_eq!(response.message.content, "Hello! I'm an AI assistant.");
    }

    #[test]
    fn test_openrouter_tool_call_arguments_parsed_as_json() {
        let body = make_response_body_with_tool_calls();
        let response = OpenRouterProvider::parse_response(&body).unwrap();

        let tool_calls = response
            .message
            .tool_calls
            .expect("Expected tool_calls to be Some");

        // Arguments should be a parsed JSON object, not a raw string
        assert!(
            tool_calls[0].arguments.is_object(),
            "Expected arguments to be a JSON object, but got: {:?}. BUG: arguments not parsed as JSON.",
            tool_calls[0].arguments
        );
        assert_eq!(tool_calls[0].arguments["operation"], "geocode");
        assert_eq!(tool_calls[0].arguments["query"], "Silla, Valencia");
    }

    #[test]
    fn test_openrouter_tool_call_invalid_arguments() {
        let body = make_response_body_invalid_arguments();
        let response = OpenRouterProvider::parse_response(&body).unwrap();

        let tool_calls = response
            .message
            .tool_calls
            .expect("Expected tool_calls to be Some");

        // Since the arguments string is invalid JSON, it should fall back to a String value
        assert!(
            tool_calls[0].arguments.is_string(),
            "Expected arguments to be a String fallback for invalid JSON, but got: {:?}",
            tool_calls[0].arguments
        );
        assert_eq!(
            tool_calls[0].arguments.as_str().unwrap(),
            "this is not valid json"
        );
    }

    // -----------------------------------------------------------------------
    // RED phase — SSE streaming tests
    //
    // These tests will FAIL because `parse_sse_event` is currently a stub that
    // calls `unimplemented!()`. They verify the expected contract for the real
    // implementation.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_sse_content_chunk() {
        let json = r#"{"choices":[{"delta":{"content":"Hello"},"index":0}]}"#;
        let mut acc = StreamAccumulator::default();

        let event = parse_sse_event(json, &mut acc)
            .expect("parse_sse_event should not return an error for valid JSON")
            .expect("expected Some(StreamEvent) for a content delta");

        match event {
            StreamEvent::Chunk(text) => {
                assert_eq!(text, "Hello", "content chunk should contain 'Hello'");
            }
            other => panic!("Expected StreamEvent::Chunk, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_sse_empty_delta_ignored() {
        let json = r#"{"choices":[{"delta":{},"index":0}]}"#;
        let mut acc = StreamAccumulator::default();

        let event = parse_sse_event(json, &mut acc)
            .expect("parse_sse_event should not return an error for an empty delta");

        assert!(
            event.is_none(),
            "empty delta should be ignored (return None), got {:?}",
            event
        );
    }

    #[test]
    fn test_parse_sse_done() {
        let json = r#"{"choices":[{"delta":{},"finish_reason":"stop","index":0}],"usage":{"prompt_tokens":10,"completion_tokens":5}}"#;
        let mut acc = StreamAccumulator::default();

        let event = parse_sse_event(json, &mut acc)
            .expect("parse_sse_event should not return an error for a finish event")
            .expect("expected Some(StreamEvent) for a finish_reason delta");

        match event {
            StreamEvent::Done(response) => {
                let usage = response
                    .usage
                    .expect("Done event should carry usage information");
                assert_eq!(usage.prompt_tokens, 10);
                assert_eq!(usage.completion_tokens, 5);
            }
            other => panic!("Expected StreamEvent::Done, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_sse_tool_call_delta() {
        let json = r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_123","type":"function","function":{"name":"get_weather","arguments":""}}]},"index":0}]}"#;
        let mut acc = StreamAccumulator::default();

        let event = parse_sse_event(json, &mut acc)
            .expect("parse_sse_event should not return an error for a tool_call delta")
            .expect("expected Some(StreamEvent) for a tool_call delta");

        match event {
            StreamEvent::ToolCall(tc) => {
                assert_eq!(tc.id, "call_123");
                assert_eq!(tc.name, "get_weather");
            }
            other => panic!("Expected StreamEvent::ToolCall, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_sse_tool_call_arguments_acumulados() {
        // First chunk: tool call header (id + name)
        let line1 = r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_456","type":"function","function":{"name":"get_weather","arguments":""}}]},"index":0}]}"#;
        // Second chunk: arguments continuation
        let line2 = r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"city\":\"Madrid\"}"}}]},"index":0}]}"#;

        let mut acc = StreamAccumulator::default();

        // Process first line
        let _event1 =
            parse_sse_event(line1, &mut acc).expect("first tool_call delta should not error");

        // Process second line
        let _event2 =
            parse_sse_event(line2, &mut acc).expect("second tool_call delta should not error");

        // After both lines are processed, the accumulator should have merged
        // the arguments. The second call may return None (just accumulated).
        assert!(
            !acc.partial_tool_calls.is_empty(),
            "expected at least one partial tool call in the accumulator"
        );

        let merged = &acc.partial_tool_calls[0];
        assert_eq!(merged.index, 0);
        assert_eq!(merged.id, "call_456");
        assert_eq!(merged.name, "get_weather");
        assert_eq!(
            merged.arguments, r#"{"city":"Madrid"}"#,
            "arguments from both chunks should be concatenated"
        );
    }

    #[test]
    fn test_parse_sse_invalid_json_returns_error() {
        // Malformed line with the `data: ` prefix that real SSE carries
        let bad_line = "data: {invalid json";
        let mut acc = StreamAccumulator::default();

        let result = parse_sse_event(bad_line, &mut acc);

        assert!(
            result.is_err(),
            "malformed SSE line should return Err, got Ok({:?})",
            result
        );
    }

    #[test]
    fn test_parse_sse_keeps_usage_for_done() {
        // Simulate multiple chunks: two content chunks then a final done event
        // with usage. The usage from the final event must propagate to Done.
        let mut acc = StreamAccumulator::default();

        // Chunk 1 — no usage
        let c1 = r#"{"choices":[{"delta":{"content":"Hello"},"index":0}]}"#;
        let _ = parse_sse_event(c1, &mut acc).expect("chunk 1 should not error");

        // Chunk 2 — still no usage
        let c2 = r#"{"choices":[{"delta":{"content":" world"},"index":0}]}"#;
        let _ = parse_sse_event(c2, &mut acc).expect("chunk 2 should not error");

        // Final event with usage
        let done = r#"{"choices":[{"delta":{},"finish_reason":"stop","index":0}],"usage":{"prompt_tokens":42,"completion_tokens":7}}"#;
        let event = parse_sse_event(done, &mut acc)
            .expect("done event should not error")
            .expect("expected Some(StreamEvent) for finish_reason");

        match event {
            StreamEvent::Done(response) => {
                let usage = response
                    .usage
                    .expect("Done event must carry usage from the last chunk");
                assert_eq!(
                    usage.prompt_tokens, 42,
                    "prompt_tokens should come from the last chunk with usage"
                );
                assert_eq!(
                    usage.completion_tokens, 7,
                    "completion_tokens should come from the last chunk with usage"
                );
            }
            other => panic!("Expected StreamEvent::Done, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // RED phase — finish_reason "tool_calls" + StreamAccumulator::finalize()
    //
    // These tests will FAIL because:
    //   - StreamAccumulator::finalize() doesn't exist yet
    //   - parse_sse_event() doesn't handle finish_reason "tool_calls"
    //     on an empty delta (no tool_calls array in the SSE line)
    // -----------------------------------------------------------------------

    #[test]
    fn test_stream_accumulator_finalize_multiple() {
        // Create accumulator with 2 partial tool calls
        let mut acc = StreamAccumulator {
            partial_tool_calls: vec![
                PartialToolCall {
                    index: 0,
                    id: "call_1".to_string(),
                    name: "weather".to_string(),
                    arguments: r#"{"city":"Madrid"}"#.to_string(),
                },
                PartialToolCall {
                    index: 1,
                    id: "call_2".to_string(),
                    name: "geo".to_string(),
                    arguments: r#"{"lat":40.4}"#.to_string(),
                },
            ],
            usage: None,
        };

        // Call finalize() — does not exist yet, so this won't compile
        let result = acc.finalize();
        let tool_calls = result.expect("expected Some(Vec<ToolCall>) for non-empty accumulator");
        assert_eq!(tool_calls.len(), 2);
        assert_eq!(tool_calls[0].name, "weather");
        assert!(
            tool_calls[0].arguments.is_object(),
            "arguments should be parsed as JSON object"
        );
        assert_eq!(tool_calls[0].arguments["city"], "Madrid");
        assert_eq!(tool_calls[1].name, "geo");
    }

    #[test]
    fn test_parse_sse_finish_reason_tool_calls_no_delta() {
        // SSE line with EMPTY delta and finish_reason="tool_calls"
        let json = r#"{"choices":[{"delta":{},"finish_reason":"tool_calls","index":0}]}"#;
        let mut acc = StreamAccumulator::default();

        let event = parse_sse_event(json, &mut acc)
            .expect("parse_sse_event should not error for finish_reason tool_calls")
            .expect("expected Some(StreamEvent) for finish_reason tool_calls");

        match event {
            StreamEvent::Done(response) => {
                assert!(
                    response.message.tool_calls.is_none(),
                    "expected tool_calls = None when accumulator is empty"
                );
            }
            other => panic!("Expected StreamEvent::Done, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_sse_finish_reason_tool_calls_with_accumulator() {
        // Pre-populate the accumulator with 1 partial tool call
        let mut acc = StreamAccumulator {
            partial_tool_calls: vec![PartialToolCall {
                index: 0,
                id: "call_123".to_string(),
                name: "get_weather".to_string(),
                arguments: r#"{"city":"Madrid"}"#.to_string(),
            }],
            usage: None,
        };

        // Process finish_reason "tool_calls" SSE line (empty delta)
        let json = r#"{"choices":[{"delta":{},"finish_reason":"tool_calls","index":0}]}"#;
        let event = parse_sse_event(json, &mut acc)
            .expect("parse_sse_event should not error")
            .expect("expected Some(StreamEvent) for finish_reason tool_calls");

        match event {
            StreamEvent::Done(response) => {
                let tool_calls = response
                    .message
                    .tool_calls
                    .expect("expected Some(tool_calls) since accumulator was pre-populated");
                assert_eq!(tool_calls.len(), 1);
                assert_eq!(tool_calls[0].name, "get_weather");
                assert!(
                    tool_calls[0].arguments.is_object(),
                    "arguments should be parsed as JSON object"
                );
                assert_eq!(tool_calls[0].arguments["city"], "Madrid");
            }
            other => panic!("Expected StreamEvent::Done, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_sse_finish_reason_tool_calls_multiple() {
        // Pre-populate accumulator with 2 partial tool calls
        let mut acc = StreamAccumulator {
            partial_tool_calls: vec![
                PartialToolCall {
                    index: 0,
                    id: "call_1".to_string(),
                    name: "weather".to_string(),
                    arguments: r#"{"city":"Madrid"}"#.to_string(),
                },
                PartialToolCall {
                    index: 1,
                    id: "call_2".to_string(),
                    name: "geo".to_string(),
                    arguments: r#"{"lat":40.4}"#.to_string(),
                },
            ],
            usage: None,
        };

        // Process finish_reason "tool_calls" SSE line
        let json = r#"{"choices":[{"delta":{},"finish_reason":"tool_calls","index":0}]}"#;
        let event = parse_sse_event(json, &mut acc)
            .expect("parse_sse_event should not error")
            .expect("expected Some(StreamEvent) for finish_reason tool_calls");

        match event {
            StreamEvent::Done(response) => {
                let tool_calls = response
                    .message
                    .tool_calls
                    .expect("expected Some(tool_calls) since accumulator had 2 calls");
                assert_eq!(tool_calls.len(), 2);
                assert_eq!(tool_calls[0].name, "weather");
                assert_eq!(tool_calls[1].name, "geo");
            }
            other => panic!("Expected StreamEvent::Done, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_sse_finish_reason_stop_still_works() {
        // Regression guard: finish_reason "stop" must still work
        let json = r#"{"choices":[{"delta":{},"finish_reason":"stop","index":0}],"usage":{"prompt_tokens":10,"completion_tokens":5}}"#;
        let mut acc = StreamAccumulator::default();

        let event = parse_sse_event(json, &mut acc)
            .expect("parse_sse_event should not return an error for finish_reason stop")
            .expect("expected Some(StreamEvent) for finish_reason stop");

        match event {
            StreamEvent::Done(response) => {
                let usage = response
                    .usage
                    .expect("Done event should carry usage information");
                assert_eq!(usage.prompt_tokens, 10);
                assert_eq!(usage.completion_tokens, 5);
            }
            other => panic!("Expected StreamEvent::Done, got {:?}", other),
        }
    }
}
