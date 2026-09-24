use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::embeddings::provider::{EmbeddingError, EmbeddingProvider};

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
    client: Client,
}

impl OpenRouterProvider {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        Self {
            api_key,
            model: model.unwrap_or_else(|| "openai/text-embedding-3-small".to_string()),
            client: Client::new(),
        }
    }
}

#[async_trait]
impl EmbeddingProvider for OpenRouterProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let url = "https://openrouter.ai/api/v1/embeddings";
        let request = OpenRouterEmbedRequest {
            model: self.model.clone(),
            input: text.to_string(),
        };

        let resp = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
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
    async fn test_openrouter_no_api_key_returns_error() {
        let provider = OpenRouterProvider::new(String::new(), None);
        let result = provider.embed("test").await;
        assert!(result.is_err());
    }
}
