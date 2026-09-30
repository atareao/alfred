use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::embeddings::provider::{EmbeddingError, EmbeddingProvider};

/// Application name sent to OpenRouter for identification.
const APP_NAME: &str = "Valet";
/// Application URL sent to OpenRouter for identification.
const APP_URL: &str = "https://github.com/atareao/valet-ai";

#[derive(Debug, Serialize)]
struct OpenRouterEmbedRequest {
    model: String,
    input: String,
}

#[derive(Debug, Deserialize)]
struct OpenRouterEmbedData {
    embedding: Vec<f32>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterEmbedResponse {
    data: Vec<OpenRouterEmbedData>,
}

pub struct OpenRouterProvider {
    api_key: String,
    model: String,
    base_url: String,
    client: Client,
}

impl OpenRouterProvider {
    /// Build a provider against the production OpenRouter base URL. The model
    /// is mandatory (no hardcoded fallback).
    pub fn new(api_key: String, model: String) -> Self {
        Self::with_base_url(api_key, model, "https://openrouter.ai/api/v1".to_string())
    }

    /// Build a provider against a custom base URL. Used by tests to target a
    /// mock server; production always uses [`OpenRouterProvider::new`].
    pub fn with_base_url(api_key: String, model: String, base_url: String) -> Self {
        Self {
            api_key,
            model,
            base_url,
            client: Client::new(),
        }
    }
}

#[async_trait]
impl EmbeddingProvider for OpenRouterProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let url = format!("{}/embeddings", self.base_url);
        let request = OpenRouterEmbedRequest {
            model: self.model.clone(),
            input: text.to_string(),
        };

        let resp = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", APP_URL)
            .header("X-Title", APP_NAME)
            .json(&request)
            .send()
            .await
            .map_err(|e| EmbeddingError::Api(e.to_string()))?;

        if resp.status() == 429 {
            return Err(EmbeddingError::RateLimited);
        }
        if !resp.status().is_success() {
            return Err(EmbeddingError::Api(format!(
                "OpenRouter returned {}",
                resp.status()
            )));
        }

        let data: OpenRouterEmbedResponse = resp
            .json()
            .await
            .map_err(|e| EmbeddingError::Api(format!("Failed to parse response: {}", e)))?;

        data.data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .ok_or_else(|| EmbeddingError::Api("Empty response".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_openrouter_no_api_key_returns_error() -> Result<(), Box<dyn std::error::Error>> {
        let provider = OpenRouterProvider::new(String::new(), "test-model".to_string());
        let result = provider.embed("test").await;
        assert!(result.is_err());
        Ok(())
    }

    // -----------------------------------------------------------------------
    // OpenRouter application identification headers (embeddings provider)
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_embed_sends_app_headers() {
        use wiremock::matchers::{any, method};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // The embeddings provider is pointed at the mock server via with_base_url.
        let provider = OpenRouterProvider::with_base_url(
            "test-key".into(),
            "test-model".into(),
            mock_server.uri(),
        );

        let captured_headers = std::sync::Arc::new(std::sync::Mutex::new(None));
        let captured = captured_headers.clone();

        Mock::given(any())
            .and(method("POST"))
            .respond_with(move |req: &wiremock::Request| {
                let mut headers = captured.lock().unwrap();
                *headers = Some(req.headers.clone());
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "data": [{
                        "embedding": [0.1, 0.2, 0.3]
                    }]
                }))
            })
            .mount(&mock_server)
            .await;

        let _ = provider.embed("test text").await;

        let headers = captured_headers
            .lock()
            .unwrap()
            .take()
            .expect("No headers captured");

        let referer = headers
            .get("HTTP-Referer")
            .or_else(|| headers.get("http-referer"))
            .or_else(|| headers.get("Http-Referer"))
            .and_then(|v| v.to_str().ok());
        assert_eq!(referer, Some("https://github.com/atareao/valet-ai"));

        let title = headers
            .get("X-Title")
            .or_else(|| headers.get("x-title"))
            .or_else(|| headers.get("X-title"))
            .and_then(|v| v.to_str().ok());
        assert_eq!(title, Some("Valet"));
    }
}
