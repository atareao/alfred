use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use std::pin::Pin;
use std::time::Duration;
use tokio_stream::Stream;

use super::provider::{
    ChatMessage, ChatRequest, ChatResponse, LLMError, LLMProvider, StreamEvent, ToolCall,
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
        // Fall back to non-streaming for now
        let result = self.chat(request).await?;
        let stream = futures::stream::once(async move { Ok(StreamEvent::Done(result)) });
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
}
