# Embeddings Spec — f4-vector-memory

## ADDED: Embedding Provider trait

```rust
// src/embeddings/provider.rs

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate an embedding vector for a text string
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError>;
    
    /// Generate embeddings for multiple texts (batched)
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError>;
}

#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("API error: {0}")]
    Api(String),
    #[error("Rate limited")]
    RateLimited,
    #[error("Timeout")]
    Timeout,
    #[error("Model not available: {0}")]
    ModelNotAvailable(String),
}
```

## ADDED: OllamaProvider

```rust
// src/embeddings/ollama.rs

pub struct OllamaProvider {
    base_url: String,  // default: http://localhost:11434
    model: String,     // default: all-minilm
    client: reqwest::Client,
}

impl OllamaProvider {
    pub fn new(base_url: Option<String>, model: Option<String>) -> Self;
}

#[async_trait]
impl EmbeddingProvider for OllamaProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError>;
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError>;
}
```

## ADDED: OpenRouterProvider

```rust
// src/embeddings/openrouter.rs

pub struct OpenRouterProvider {
    api_key: String,
    model: String,  // default: openai/text-embedding-3-small
    client: reqwest::Client,
}

impl OpenRouterProvider {
    pub fn new(api_key: String, model: Option<String>) -> Self;
}

#[async_trait]
impl EmbeddingProvider for OpenRouterProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError>;
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError>;
}
```

## ADDED: Embedding module

```rust
// src/embeddings/mod.rs

pub mod provider;
pub mod ollama;
pub mod openrouter;

pub use provider::EmbeddingProvider;

/// Create the default embedding provider based on config
pub fn create_provider(config: &EmbeddingConfig) -> Box<dyn EmbeddingProvider>;

pub struct EmbeddingConfig {
    pub provider: String,         // "ollama" | "openrouter"
    pub ollama_url: String,
    pub ollama_model: String,
    pub openrouter_key: String,
    pub openrouter_model: String,
}
```

## Scenarios (BDD)

### Scenario: OllamaProvider generates embeddings
- **Given** OllamaProvider is configured with a valid URL
- **When** embed("Hello world") is called
- **Then** returns a Vec<f32> with 384 dimensions (all-minilm) or 1536 (nomic)

### Scenario: OpenRouterProvider generates embeddings
- **Given** OpenRouterProvider is configured with a valid API key
- **When** embed("Test text") is called
- **Then** returns a Vec<f32> with 1536 dimensions

### Scenario: EmbeddingProvider handles errors gracefully
- **Given** the API is unreachable
- **When** embed() is called
- **Then** returns EmbeddingError::Api or EmbeddingError::Timeout