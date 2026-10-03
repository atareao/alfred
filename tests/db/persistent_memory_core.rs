//! Characterization of the Layer C (persistent memory) baseline — Bloque 1 of
//! the `persistent-memory-core` change.
//!
//! These tests document the state of the code *before* the Capa C migration and
//! the worker pass are introduced:
//!
//! 1. there is no `persistent_memory` table yet, and
//! 2. the episodic worker performs a **single** extraction (a Layer B card) and
//!    writes no Layer C state.
//!
//! The reserved persistent-memory slot in the system message (its insertion
//! point) already exists but is intentionally empty; that behaviour is pinned
//! by `orchestrator::agent`'s own `compose_system_message_reserves_*` tests, so
//! it is referenced here rather than duplicated.

use async_trait::async_trait;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;

use valet::embeddings::provider::EmbeddingError;
use valet::embeddings::EmbeddingProvider;
use valet::llm::provider::{
    ChatMessage, ChatRequest, ChatResponse, LLMError, LLMProvider, StreamEvent, TokenUsage,
};
use valet::workers::episodic_memory::{EpisodicMemoryConfig, EpisodicMemoryWorker};

async fn setup() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(":memory:")
                .create_if_missing(true),
        )
        .await
        .expect("in-memory pool");
    valet::db::schema::run_migrations(&pool)
        .await
        .expect("migrations");
    pool
}

async fn table_exists(pool: &SqlitePool, name: &str) -> bool {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name = ?1",
    )
    .bind(name)
    .fetch_one(pool)
    .await
    .expect("sqlite_master query")
        > 0
}

/// Number of rows in `persistent_memory`, or `0` when the table does not exist
/// yet. Keeps the worker characterization truthful both before and after the
/// Capa C migration lands.
async fn persistent_memory_rows(pool: &SqlitePool) -> i64 {
    if !table_exists(pool, "persistent_memory").await {
        return 0;
    }
    sqlx::query_scalar("SELECT COUNT(*) FROM persistent_memory")
        .fetch_one(pool)
        .await
        .expect("count persistent_memory")
}

/// LLM double that counts calls and always returns a parseable card.
struct CountingLLM {
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl LLMProvider for CountingLLM {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, LLMError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(ChatResponse {
            message: ChatMessage {
                role: "assistant".into(),
                content: "\
- FECHA/CONTEXTO: caracterización de la línea base
- TEMAS TRATADOS: nodo aislado
- HECHOS Y DECISIONES: una sola extracción
- SÍNTESIS: el worker produce únicamente la ficha episódica (Capa B)"
                    .into(),
                tool_calls: None,
                tool_result: None,
                tool_call_id: None,
            },
            usage: Some(TokenUsage {
                prompt_tokens: 10,
                completion_tokens: 10,
                cached_tokens: 0,
                reasoning_tokens: 0,
                cost: 0.0,
            }),
        })
    }

    async fn chat_stream(
        &self,
        _request: ChatRequest,
    ) -> Result<
        Pin<Box<dyn tokio_stream::Stream<Item = Result<StreamEvent, LLMError>> + Send>>,
        LLMError,
    > {
        unimplemented!("chat_stream is not used in this test")
    }
}

/// Embedding double returning a valid 1024-dim vector.
struct FixedEmbedding;

#[async_trait]
impl EmbeddingProvider for FixedEmbedding {
    async fn embed(&self, _input: &str) -> Result<Vec<f32>, EmbeddingError> {
        Ok(vec![0.1f32; 1024])
    }
}

/// BLOCK 1 CHARACTERIZATION → **CHANGED ON PURPOSE (Bloque 2)**: the Capa C
/// migration now creates the `persistent_memory` table, so the baseline's
/// "table absent" assertion is retired and replaced by the counterpart: the
/// table exists with the documented shape.
#[tokio::test]
async fn characterization_persistent_memory_table_present_after_migration() {
    let pool = setup().await;
    assert!(
        table_exists(&pool, "persistent_memory").await,
        "after the Capa C migration the persistent_memory table must exist"
    );
}

/// BLOCK 1 CHARACTERIZATION: the episodic worker makes exactly **one** LLM
/// extraction and writes only the Layer B card (`memory` + `vec_memory` + the
/// index mark). It writes no Layer C (persistent) state.
///
/// This test is deliberately robust to the Capa C table appearing in Bloque 2:
/// the invariant it pins for Bloque 1 is "no Layer C write", not "no table".
#[tokio::test]
async fn characterization_worker_writes_only_episodic_card() {
    let pool = setup().await;
    let now = chrono::Utc::now().to_rfc3339();

    // 5 unindexed messages × 500 tokens = 2500 ≥ default batch_tokens (2000).
    for i in 0..5 {
        sqlx::query(
            "INSERT INTO messages (id, role, content, tokens_count, is_indexed, created_at)
             VALUES (?1, 'user', ?2, 500, 0, ?3)",
        )
        .bind(format!("m{i}"))
        .bind(format!("Message {i}"))
        .bind(&now)
        .execute(&pool)
        .await
        .expect("insert message");
    }

    let calls = Arc::new(AtomicUsize::new(0));
    let (memory_tx, memory_rx) = tokio::sync::mpsc::channel::<()>(16);
    let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel::<()>(1);

    let _handle = EpisodicMemoryWorker::start(
        pool.clone(),
        Arc::new(CountingLLM {
            calls: calls.clone(),
        }),
        Arc::new(FixedEmbedding),
        memory_rx,
        shutdown_rx,
        EpisodicMemoryConfig {
            poll_interval_minutes: 999,
            ..Default::default()
        },
    );

    memory_tx.send(()).await.expect("signal");
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Exactly ONE extraction: a single Layer B card.
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "baseline: the worker performs a single extraction (Layer B card)"
    );

    let memory_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memory")
        .fetch_one(&pool)
        .await
        .expect("count memory");
    assert_eq!(memory_rows, 1, "exactly one Layer B card is written");

    let vec_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM vec_memory")
        .fetch_one(&pool)
        .await
        .expect("count vec_memory");
    assert_eq!(vec_rows, 1, "the card's embedding is written");

    // The primary batch is a prefix bounded by `batch_tokens` (4 × 500 = 2000),
    // so exactly those four messages are marked as indexed in this pass.
    let unindexed: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE is_indexed = 0")
        .fetch_one(&pool)
        .await
        .expect("count unindexed");
    assert_eq!(
        unindexed, 1,
        "the bounded primary batch is marked as indexed"
    );

    let indexed: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE is_indexed = 1")
        .fetch_one(&pool)
        .await
        .expect("count indexed");
    assert_eq!(indexed, 4, "four origin messages are marked as indexed");

    // No Layer C state is produced by the baseline worker.
    assert_eq!(
        persistent_memory_rows(&pool).await,
        0,
        "baseline: the worker writes no persistent (Layer C) state"
    );

    let _ = shutdown_tx.send(());
}
