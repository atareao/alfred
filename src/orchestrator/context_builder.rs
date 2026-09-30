use crate::db::repos::memory::MemoryRepo;
use crate::embeddings::EmbeddingProvider;
use crate::llm::provider::ChatMessage;
use crate::models::Memory;
use crate::orchestrator::context_classifier::ContextStrategy;
use sqlx::SqlitePool;
use std::sync::Arc;

pub struct BuiltContext {
    pub system_prompt: String,
    pub messages: Vec<ChatMessage>,
    pub token_estimate: usize,
    pub rag_memories: Vec<String>,
    pub session_summary: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ContextError {
    #[error("Profile not found")]
    ProfileNotFound,
    #[error("Search error: {0}")]
    SearchError(String),
    #[error("Window error: {0}")]
    WindowError(String),
}

pub struct ContextBuilder {
    pub pool: Option<SqlitePool>,
    pub provider: Option<Arc<dyn EmbeddingProvider>>,
    pub rag_budget_tokens: usize,
}

impl Default for ContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextBuilder {
    pub fn new() -> Self {
        Self {
            pool: None,
            provider: None,
            rag_budget_tokens: 2000,
        }
    }

    pub async fn build(
        &self,
        strategy: ContextStrategy,
        _profile_id: &str,
        user_message: &str,
    ) -> Result<BuiltContext, ContextError> {
        match strategy {
            ContextStrategy::SlidingWindow => Ok(BuiltContext {
                system_prompt: "You are Valet, a helpful AI assistant.".into(),
                messages: vec![],
                token_estimate: 500,
                rag_memories: vec![],
                session_summary: None,
            }),
            ContextStrategy::Historical => Ok(BuiltContext {
                system_prompt: "You are Valet, analyzing historical data.".into(),
                messages: vec![],
                token_estimate: 5000,
                rag_memories: vec![],
                session_summary: None,
            }),
            ContextStrategy::RAG => {
                let rag_memories = self.build_rag_memories(user_message).await;
                let token_estimate = rag_memories.iter().map(|m| m.len()).sum::<usize>() + 500;
                Ok(BuiltContext {
                    system_prompt: "You are Valet, using RAG context.".into(),
                    messages: vec![],
                    token_estimate,
                    rag_memories,
                    session_summary: None,
                })
            }
        }
    }

    /// Perform a real vector-similarity search.
    ///
    /// Returns `Vec::new()` (never placeholder memories) when the pool or the
    /// embedding provider is missing, or when the embedding/search fails.
    async fn build_rag_memories(&self, user_message: &str) -> Vec<String> {
        let (Some(pool), Some(provider)) = (&self.pool, &self.provider) else {
            tracing::warn!("RAG: pool or embedding provider not configured; returning no memories");
            return Vec::new();
        };

        let embedding = match provider.embed(user_message).await {
            Ok(emb) => emb,
            Err(e) => {
                tracing::warn!("RAG: embedding generation failed: {e}");
                return Vec::new();
            }
        };

        match MemoryRepo::search_by_vector(pool, &embedding, 10, self.rag_budget_tokens).await {
            Ok(memories) => memories.into_iter().map(|m| format_memory(&m)).collect(),
            Err(e) => {
                tracing::warn!("RAG: vector search failed: {e}");
                Vec::new()
            }
        }
    }
}

/// Format a `Memory` card into the `[{tags}] {content}` display string.
fn format_memory(m: &Memory) -> String {
    let tags = m
        .metadata
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    format!("[{tags}] {}", m.content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::run_migrations;
    use crate::embeddings::provider::EmbeddingError;
    use async_trait::async_trait;
    use sqlx::sqlite::SqlitePoolOptions;

    /// In-memory pool with all migrations applied.
    async fn setup_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("failed to create in-memory pool");
        run_migrations(&pool).await.expect("migrations failed");
        pool
    }

    // ─── Mock embedding provider ─────────────────────────────────────────

    struct MockEmbedProvider;

    #[async_trait]
    impl EmbeddingProvider for MockEmbedProvider {
        async fn embed(&self, _input: &str) -> Result<Vec<f32>, EmbeddingError> {
            Ok(vec![0.1, 0.2, 0.3])
        }
    }

    // ─── Existing tests (must stay GREEN) ─────────────────────────────────

    #[tokio::test]
    async fn test_sliding_window_context() -> Result<(), Box<dyn std::error::Error>> {
        let builder = ContextBuilder::new();
        let ctx = builder
            .build(ContextStrategy::SlidingWindow, "profile-1", "hello")
            .await?;
        assert!(ctx.system_prompt.contains("Valet"));
        assert!(ctx.token_estimate <= 2000);
        Ok(())
    }

    #[tokio::test]
    async fn test_historical_context() -> Result<(), Box<dyn std::error::Error>> {
        let builder = ContextBuilder::new();
        let ctx = builder
            .build(ContextStrategy::Historical, "profile-1", "history")
            .await?;
        assert!(ctx.token_estimate >= 1000);
        Ok(())
    }

    #[tokio::test]
    async fn test_rag_context_without_pool_is_empty() -> Result<(), Box<dyn std::error::Error>> {
        let builder = ContextBuilder::new();
        let ctx = builder
            .build(ContextStrategy::RAG, "profile-1", "search")
            .await?;
        // No pool/provider → no placeholder memories, just an empty vec.
        assert!(ctx.rag_memories.is_empty());
        Ok(())
    }

    // ─── New RAG tests ───────────────────────────────────────────────────

    #[tokio::test]
    async fn test_rag_without_pool() {
        let builder = ContextBuilder::new();
        let ctx = builder
            .build(ContextStrategy::RAG, "profile-1", "any")
            .await
            .expect("build should succeed even without pool");
        // Without pool → no placeholder memories.
        assert!(
            ctx.rag_memories.is_empty(),
            "expected empty rag_memories when pool is None"
        );
    }

    #[tokio::test]
    async fn test_rag_with_pool_no_provider() {
        let pool = setup_pool().await;
        let builder = ContextBuilder {
            pool: Some(pool),
            provider: None,
            rag_budget_tokens: 2000,
        };
        let ctx = builder
            .build(ContextStrategy::RAG, "profile-1", "any")
            .await
            .expect("build should succeed with pool but no provider");
        // No provider → no placeholder memories.
        assert!(
            ctx.rag_memories.is_empty(),
            "expected empty rag_memories when provider is None"
        );
    }

    #[tokio::test]
    async fn test_rag_empty_db() {
        let pool = setup_pool().await;
        let provider = Arc::new(MockEmbedProvider);
        let builder = ContextBuilder {
            pool: Some(pool),
            provider: Some(provider),
            rag_budget_tokens: 2000,
        };
        let ctx = builder
            .build(ContextStrategy::RAG, "profile-1", "search")
            .await
            .expect("build should succeed with empty vec_memory");
        // No memories stored → search_by_vector returns empty vec
        assert!(
            ctx.rag_memories.is_empty(),
            "expected empty rag_memories when vec_memory is empty"
        );
    }

    /// Insert an embedding row directly into `vec_memory` for tests.
    async fn insert_embedding(pool: &SqlitePool, id: &str, embedding: &[f32]) {
        let json = serde_json::to_string(embedding).expect("failed to serialize embedding");
        sqlx::query("INSERT INTO vec_memory (id, embedding) VALUES (?1, ?2)")
            .bind(id)
            .bind(&json)
            .execute(pool)
            .await
            .expect("failed to insert vec_memory row");
    }

    #[tokio::test]
    async fn test_rag_with_pool_and_provider_returns_formatted_memories() {
        let pool = setup_pool().await;

        // A real memory card with tags.
        let metadata = serde_json::json!({"tags": ["rust", "backend"]});
        let mem = MemoryRepo::create(&pool, "User likes Rust", 10, &metadata)
            .await
            .expect("create memory should succeed");

        // Embedding matches the mock provider's output dimension (3).
        insert_embedding(&pool, &mem.id, &[0.1, 0.2, 0.3]).await;

        let builder = ContextBuilder {
            pool: Some(pool),
            provider: Some(Arc::new(MockEmbedProvider)),
            rag_budget_tokens: 2000,
        };
        let ctx = builder
            .build(ContextStrategy::RAG, "profile-1", "what does the user like")
            .await
            .expect("build should succeed with pool + provider");

        assert_eq!(
            ctx.rag_memories,
            vec!["[rust, backend] User likes Rust".to_string()],
            "rag_memories should contain the formatted `[tags] content` memory"
        );
    }
}
