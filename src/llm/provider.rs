use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::pin::Pin;
use thiserror::Error;
use tokio_stream::Stream;

/// A message in a chat conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// A tool call instruction from the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

/// Definition of a tool that can be provided to the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

/// Request payload for an LLM chat completion.
#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub tools: Option<Vec<ToolDef>>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: bool,
}

/// Response from an LLM chat completion.
#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub message: ChatMessage,
    pub usage: Option<TokenUsage>,
}

/// Token usage statistics.
#[derive(Debug, Clone)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

/// Events emitted during streaming chat completions.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// A text chunk from the stream.
    Chunk(String),
    /// The stream is complete with the final response.
    Done(ChatResponse),
    /// A tool call was requested during streaming.
    ToolCall(ToolCall),
}

/// Errors that can occur during LLM operations.
#[derive(Error, Debug, Clone)]
pub enum LLMError {
    #[error("HTTP error: {0}")]
    HttpError(String),
    #[error("Rate limited, retry after {retry_after}s")]
    RateLimited { retry_after: u64 },
    #[error("Timeout: {0}")]
    Timeout(String),
    #[error("Auth error: {0}")]
    AuthError(String),
    #[error("Model not available: {0}")]
    ModelNotAvailable(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<reqwest::Error> for LLMError {
    fn from(e: reqwest::Error) -> Self {
        LLMError::HttpError(e.to_string())
    }
}

/// Trait that all LLM providers must implement.
///
/// Provides chat completions (both streaming and non-streaming) and embeddings.
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Send a non-streaming chat completion request.
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, LLMError>;

    /// Send a streaming chat completion request.
    ///
    /// Returns a stream of [`StreamEvent`] values.
    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, LLMError>> + Send>>, LLMError>;

    /// Generate an embedding vector for the given input text.
    async fn embed(&self, input: &str) -> Result<Vec<f32>, LLMError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_creation() {
        let msg = ChatMessage {
            role: "user".into(),
            content: "Hello".into(),
            tool_calls: None,
            tool_result: None,
            tool_call_id: None,
        };
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_tool_call_creation() {
        let tc = ToolCall {
            id: "call_1".into(),
            name: "get_weather".into(),
            arguments: serde_json::json!({"city": "Madrid"}),
        };
        assert_eq!(tc.name, "get_weather");
    }

    #[test]
    fn test_chat_request_defaults() {
        let req = ChatRequest {
            model: "test-model".into(),
            messages: vec![],
            tools: None,
            temperature: None,
            max_tokens: None,
            stream: false,
        };
        assert!(!req.stream);
    }

    #[test]
    fn test_llm_error_display() {
        let err = LLMError::HttpError("connection failed".into());
        assert!(err.to_string().contains("connection failed"));
        let err = LLMError::RateLimited { retry_after: 30 };
        assert!(err.to_string().contains("30"));
    }

    #[test]
    fn test_llm_error_clone() {
        let err = LLMError::Timeout("slow".into());
        let cloned = err.clone();
        assert!(matches!(cloned, LLMError::Timeout(_)));
    }
}
