use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::embeddings::provider::{EmbeddingError, EmbeddingProvider};

#[derive(Debug, Serialize)]
struct OllamaEmbedRequest {
    model: String,
    prompt: String,
}

#[derive(Debug, Deserialize)]
struct OllamaEmbedResponse {
    embedding: Vec<f32>,
}

pub struct OllamaProvider {
    base_url: String,
    model: String,
    client: Client,
}

impl OllamaProvider {
    pub fn new(base_url: Option<String>, model: Option<String>) -> Self {
        Self {
            base_url: base_url.unwrap_or_else(|| "http://localhost:11434".to_string()),
            model: model.unwrap_or_else(|| "all-minilm".to_string()),
            client: Client::new(),
        }
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let url = format!("{}/api/embeddings", self.base_url);
        let request = OllamaEmbedRequest {
            model: self.model.clone(),
            prompt: text.to_string(),
        };

        let resp = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| EmbeddingError::Api(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(EmbeddingError::Api(format!(
                "Ollama returned {}",
                resp.status()
            )));
        }

        let data: OllamaEmbedResponse = resp
            .json()
            .await
            .map_err(|e| EmbeddingError::Api(format!("Failed to parse response: {}", e)))?;

        Ok(data.embedding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ollama_embed_timeout_returns_error() {
        // Point to unreachable address to test error handling
        let provider = OllamaProvider::new(Some("http://localhost:1".to_string()), None);
        let result = provider.embed("test").await;
        assert!(result.is_err());
    }
}
