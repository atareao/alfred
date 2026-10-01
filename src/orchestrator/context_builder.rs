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
            rag_budget_tokens: 800,
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

    /// Perform a real vector-similarity search and assemble the memory block.
    ///
    /// Retrieval (KNN, similarity threshold, temporal decay, ordering) is done
    /// by [`MemoryRepo::search_by_vector`]. This method owns only the **prompt
    /// budget**: it reads `RAG_BUDGET_TOKENS` from `settings` on every call
    /// (falling back to the configured `rag_budget_tokens`) and accumulates
    /// `tokens_count` until the next card would overflow the budget, at which
    /// point it stops with `break` — a card that does not fit must not let a
    /// later, less relevant card slip in for being smaller (design D6).
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

        let memories = match MemoryRepo::search_by_vector(pool, &embedding).await {
            Ok(memories) => memories,
            Err(e) => {
                tracing::warn!("RAG: vector search failed: {e}");
                return Vec::new();
            }
        };

        let budget_tokens = read_rag_budget_tokens(pool)
            .await
            .unwrap_or(self.rag_budget_tokens);

        let mut results = Vec::new();
        let mut running_tokens: usize = 0;
        for memory in memories {
            if running_tokens + memory.tokens_count > budget_tokens {
                // Step 7 (D6): stop, do NOT keep scanning for a smaller card.
                break;
            }
            running_tokens += memory.tokens_count;
            results.push(format_memory(&memory));
        }
        results
    }
}

/// Read `RAG_BUDGET_TOKENS` from `settings`, so it can be changed in the UI
/// without a restart. Returns `None` when it is absent or unparseable, so the
/// caller can fall back to the statically configured value.
async fn read_rag_budget_tokens(pool: &SqlitePool) -> Option<usize> {
    crate::db::repos::settings::SettingsRepo::get(pool, "RAG_BUDGET_TOKENS")
        .await
        .ok()
        .flatten()
        .and_then(|v| v.trim().parse::<usize>().ok())
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

    /// Pad a leading slice to the 1024 dimensions the `vec0` table declares.
    fn v1024(leading: &[f32]) -> Vec<f32> {
        let mut v = leading.to_vec();
        v.resize(1024, 0.0);
        v
    }

    struct MockEmbedProvider;

    #[async_trait]
    impl EmbeddingProvider for MockEmbedProvider {
        async fn embed(&self, _input: &str) -> Result<Vec<f32>, EmbeddingError> {
            Ok(v1024(&[0.1, 0.2, 0.3]))
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
    ///
    /// CHANGED ON PURPOSE (invariant exception 2): stored through
    /// `vec_f32(?)` with the declared 1024 dimensions, instead of raw JSON text
    /// in the old regular table.
    async fn insert_embedding(pool: &SqlitePool, id: &str, embedding: &[f32]) {
        let json = serde_json::to_string(embedding).expect("failed to serialize embedding");
        sqlx::query("INSERT INTO vec_memory (id, embedding) VALUES (?1, vec_f32(?2))")
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
        insert_embedding(&pool, &mem.id, &v1024(&[0.1, 0.2, 0.3])).await;

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

    // ─── Characterization: the strategy governs memory (task 1.2) ──────────
    //
    // These tests pin down CURRENT behaviour, which the
    // `episodic-memory-injection` change will deliberately replace: today the
    // `ContextStrategy` decides whether episodic memory is retrieved at all.
    // `SlidingWindow` and `Historical` hard-code `rag_memories: vec![]` inside
    // `ContextBuilder::build`, so with an *identical* builder (same pool, same
    // provider) and *identical* rows in `memory` + `vec_memory`, only `RAG`
    // returns the card. The tests below prove exactly that difference.
    //
    // ─── The three and only exceptions to the invariant (task 1.3) ──────────
    //
    // These are the ONLY test assertions allowed to change on purpose:
    //   1. `test_rag_with_pool_and_provider_returns_formatted_memories`, which
    //      pins the `[{tags}] {content}` format.
    //   2. Any test (here and in `db::repos::memory`) that assumes
    //      `vec_memory.embedding` stores JSON text.
    //   3. `test_doc_override` and `test_classify_with_doc` in
    //      `context_classifier`, which disappear along with `!doc`.
    // Any OTHER test that breaks when implementing the change is a regression,
    // not an update.

    /// Build a `ContextBuilder` wired with a pool + provider and seed exactly
    /// one memory card (with tags) plus its `vec_memory` embedding row. The
    /// same builder + state is then reused across all three strategies.
    async fn builder_with_one_stored_memory() -> ContextBuilder {
        let pool = setup_pool().await;

        let metadata = serde_json::json!({"tags": ["rust", "backend"]});
        let mem = MemoryRepo::create(&pool, "User likes Rust", 10, &metadata)
            .await
            .expect("create memory should succeed");

        // Embedding matches the mock provider's output dimension (3).
        insert_embedding(&pool, &mem.id, &v1024(&[0.1, 0.2, 0.3])).await;

        ContextBuilder {
            pool: Some(pool),
            provider: Some(Arc::new(MockEmbedProvider)),
            rag_budget_tokens: 2000,
        }
    }

    /// CURRENT: `SlidingWindow` never retrieves episodic memory, even though
    /// the builder has a pool, a provider and a matching stored memory.
    #[tokio::test]
    async fn characterization_sliding_window_hardcodes_empty_memories() {
        let builder = builder_with_one_stored_memory().await;
        let ctx = builder
            .build(
                ContextStrategy::SlidingWindow,
                "profile-1",
                "what does the user like",
            )
            .await
            .expect("build should succeed");
        assert!(
            ctx.rag_memories.is_empty(),
            "CURRENT behaviour: SlidingWindow hard-codes rag_memories to empty \
             even with pool, provider and a matching memory present"
        );
    }

    /// CURRENT: `Historical` never retrieves episodic memory, same as above.
    #[tokio::test]
    async fn characterization_historical_hardcodes_empty_memories() {
        let builder = builder_with_one_stored_memory().await;
        let ctx = builder
            .build(
                ContextStrategy::Historical,
                "profile-1",
                "what does the user like",
            )
            .await
            .expect("build should succeed");
        assert!(
            ctx.rag_memories.is_empty(),
            "CURRENT behaviour: Historical hard-codes rag_memories to empty \
             even with pool, provider and a matching memory present"
        );
    }

    /// CURRENT: with the *same* builder and the *same* stored rows, only `RAG`
    /// returns the memory. This is the invariant the change will break.
    #[tokio::test]
    async fn characterization_only_rag_returns_the_stored_memory() {
        let builder = builder_with_one_stored_memory().await;
        let ctx = builder
            .build(ContextStrategy::RAG, "profile-1", "what does the user like")
            .await
            .expect("build should succeed");
        assert_eq!(
            ctx.rag_memories,
            vec!["[rust, backend] User likes Rust".to_string()],
            "CURRENT behaviour: RAG is the only strategy that retrieves memory"
        );
    }

    /// `BuiltContext::system_prompt` is **vestigial** in production: `agent.rs`
    /// ignores it and reads the real prompt from `settings.system_prompt`. This
    /// test only records the dead values so their removal is visible; it does
    /// not assert anything about what the LLM receives.
    #[tokio::test]
    async fn characterization_built_context_system_prompt_is_vestigial() {
        let builder = builder_with_one_stored_memory().await;

        let sliding = builder
            .build(ContextStrategy::SlidingWindow, "profile-1", "hi")
            .await
            .expect("build should succeed");
        let historical = builder
            .build(ContextStrategy::Historical, "profile-1", "hi")
            .await
            .expect("build should succeed");
        let rag = builder
            .build(ContextStrategy::RAG, "profile-1", "hi")
            .await
            .expect("build should succeed");

        // NOTE: `agent.rs` never sends these strings to the LLM — it reads
        // `settings.system_prompt` instead. Kept only to document the field.
        assert_eq!(
            sliding.system_prompt,
            "You are Valet, a helpful AI assistant."
        );
        assert_eq!(
            historical.system_prompt,
            "You are Valet, analyzing historical data."
        );
        assert_eq!(rag.system_prompt, "You are Valet, using RAG context.");
    }

    // ─── 6.4 / block 5.4: the prompt budget cuts with `break` ────────────────

    /// Provider that embeds every input to the x-axis, so a stored vector's
    /// cosine similarity to the query is exactly its first component.
    struct AxisEmbedProvider;

    #[async_trait]
    impl EmbeddingProvider for AxisEmbedProvider {
        async fn embed(&self, _input: &str) -> Result<Vec<f32>, EmbeddingError> {
            Ok(v1024(&[1.0, 0.0, 0.0]))
        }
    }

    /// A unit vector at cosine similarity `s` to `(1, 0, 0)`, padded to 1024.
    fn cosine_vector(s: f32) -> Vec<f32> {
        v1024(&[s, (1.0 - s * s).max(0.0).sqrt(), 0.0])
    }

    async fn set_setting(pool: &SqlitePool, key: &str, value: &str) {
        crate::db::repos::settings::SettingsRepo::set(pool, key, value)
            .await
            .expect("failed to set setting");
    }

    /// Seed a card with its own vector and token count.
    async fn seed_card(pool: &SqlitePool, content: &str, tokens: usize, similarity: f32) -> String {
        let mem = MemoryRepo::create(pool, content, tokens, &serde_json::json!({}))
            .await
            .expect("create memory should succeed");
        insert_embedding(pool, &mem.id, &cosine_vector(similarity)).await;
        mem.id
    }

    fn axis_builder(pool: SqlitePool) -> ContextBuilder {
        ContextBuilder {
            pool: Some(pool),
            provider: Some(Arc::new(AxisEmbedProvider)),
            // Reserve value; the live budget is read from `settings`.
            rag_budget_tokens: 800,
        }
    }

    /// `RAG_BUDGET_TOKENS = 300` and two 237-token cards: the first fits, the
    /// second does not, and the accumulation stops there.
    #[tokio::test]
    async fn test_rag_budget_includes_first_card_not_second() {
        let pool = setup_pool().await;
        set_setting(&pool, "RAG_BUDGET_TOKENS", "300").await;

        seed_card(&pool, "First", 237, 0.99).await;
        seed_card(&pool, "Second", 237, 0.98).await;

        let ctx = axis_builder(pool)
            .build(ContextStrategy::RAG, "profile-1", "query")
            .await
            .expect("build should succeed");

        assert_eq!(
            ctx.rag_memories.len(),
            1,
            "only the first 237-token card fits in a 300-token budget"
        );
        assert!(
            ctx.rag_memories[0].contains("First"),
            "the fitting card must be the first one, got {:?}",
            ctx.rag_memories
        );
    }

    /// The spec scenario: budget 800 with cards of 347/307/248/245/210/164/138
    /// tokens → exactly 2 cards (347 + 307). The 138-token card must NOT slip
    /// in by skipping the 248-token one — that would be the `continue` bug.
    #[tokio::test]
    async fn test_rag_budget_stops_at_first_card_that_does_not_fit() {
        let pool = setup_pool().await;
        set_setting(&pool, "RAG_BUDGET_TOKENS", "800").await;

        // Decreasing similarity ⇒ deterministic relevance order.
        for (content, tokens, similarity) in [
            ("a", 347usize, 0.99f32),
            ("b", 307, 0.98),
            ("c", 248, 0.97),
            ("d", 245, 0.96),
            ("e", 210, 0.95),
            ("f", 164, 0.94),
            ("g", 138, 0.93),
        ] {
            seed_card(&pool, content, tokens, similarity).await;
        }

        let ctx = axis_builder(pool)
            .build(ContextStrategy::RAG, "profile-1", "query")
            .await
            .expect("build should succeed");

        assert_eq!(
            ctx.rag_memories.len(),
            2,
            "347 + 307 fit; the next card does not and nothing later may slip in, got {:?}",
            ctx.rag_memories
        );
        assert!(
            ctx.rag_memories[0].contains('a') && ctx.rag_memories[1].contains('b'),
            "the two fitting cards must be the 347 and 307 ones, got {:?}",
            ctx.rag_memories
        );
        assert!(
            !ctx.rag_memories.iter().any(|m| m.contains('g')),
            "the small 138-token card must NOT be included"
        );
    }

    /// The budget is read from `settings` on every call, so changing it takes
    /// effect without a restart and is not governed by the reserve field.
    #[tokio::test]
    async fn test_rag_budget_read_from_settings_takes_effect_without_restart() {
        let pool = setup_pool().await;

        seed_card(&pool, "One", 300, 0.99).await;
        seed_card(&pool, "Two", 300, 0.98).await;

        let builder = axis_builder(pool.clone());

        // Reserve field says 800, but settings says 300 → only one card.
        set_setting(&pool, "RAG_BUDGET_TOKENS", "300").await;
        let tight = builder
            .build(ContextStrategy::RAG, "profile-1", "query")
            .await
            .expect("build should succeed");
        assert_eq!(tight.rag_memories.len(), 1);

        // Raising the setting, same builder, no restart → both cards.
        set_setting(&pool, "RAG_BUDGET_TOKENS", "800").await;
        let wide = builder
            .build(ContextStrategy::RAG, "profile-1", "query")
            .await
            .expect("build should succeed");
        assert_eq!(wide.rag_memories.len(), 2);
    }
}
