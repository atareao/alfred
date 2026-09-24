pub mod ollama;
pub mod openrouter;
pub mod provider;

pub use provider::EmbeddingProvider;

/// Configuration for embedding providers
#[derive(Clone)]
pub struct EmbeddingConfig {
    pub provider: String, // "ollama" | "openrouter"
    pub ollama_url: String,
    pub ollama_model: String,
    pub openrouter_api_key: String,
    pub openrouter_model: String,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: "ollama".to_string(),
            ollama_url: "http://localhost:11434".to_string(),
            ollama_model: "all-minilm".to_string(),
            openrouter_api_key: String::new(),
            openrouter_model: "openai/text-embedding-3-small".to_string(),
        }
    }
}

/// Create the default embedding provider based on config
pub fn create_provider(config: &EmbeddingConfig) -> Box<dyn EmbeddingProvider> {
    match config.provider.as_str() {
        "openrouter" => Box::new(openrouter::OpenRouterProvider::new(
            config.openrouter_api_key.clone(),
            Some(config.openrouter_model.clone()),
        )),
        _ => Box::new(ollama::OllamaProvider::new(
            Some(config.ollama_url.clone()),
            Some(config.ollama_model.clone()),
        )),
    }
}
