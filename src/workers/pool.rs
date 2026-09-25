use crate::config::Config;
use crate::db::DbPool;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;

use crate::llm::provider::LLMProvider;

/// Container for all background worker tasks.
///
/// Each field holds an optional [`JoinHandle`] for the corresponding worker.
/// When `shutdown()` is called, a signal is broadcast to all workers and
/// all handles are aborted.
pub struct WorkerPool {
    pub briefing: Option<JoinHandle<()>>,
    pub conflict_detector: Option<JoinHandle<()>>,
    pub travel_prep: Option<JoinHandle<()>>,
    pub memory_consolidator: Option<JoinHandle<()>>,
    pub collapse: Option<JoinHandle<()>>,
    pub collapse_tx: Option<mpsc::Sender<String>>,
    shutdown_tx: Option<broadcast::Sender<()>>,
}

impl WorkerPool {
    /// Start all workers. Each worker runs on a [`tokio::time::interval`].
    ///
    /// A broadcast channel is created so all workers can be gracefully
    /// stopped. Worker bodies are placeholders that log a tick — real
    /// logic will be wired in later tasks (6.2–6.5).
    pub fn start(db: DbPool, _config: &Config, llm_provider: Arc<dyn LLMProvider>) -> Self {
        let (shutdown_tx, _) = broadcast::channel::<()>(1);

        // ── Briefing worker ────────────────────────────────────────────
        let briefing = {
            let mut shutdown_rx = shutdown_tx.subscribe();
            let db = db.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            tracing::info!("[WorkerPool] Briefing worker tick");
                            let _ = &db;
                        }
                        _ = shutdown_rx.recv() => {
                            tracing::info!("[WorkerPool] Briefing worker shutting down");
                            break;
                        }
                    }
                }
            })
        };

        // ── Conflict detector worker ────────────────────────────────────
        let conflict_detector = {
            let mut shutdown_rx = shutdown_tx.subscribe();
            let db = db.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(120));
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            tracing::info!("[WorkerPool] Conflict-detector worker tick");
                            let _ = &db;
                        }
                        _ = shutdown_rx.recv() => {
                            tracing::info!("[WorkerPool] Conflict-detector worker shutting down");
                            break;
                        }
                    }
                }
            })
        };

        // ── Travel prep worker ──────────────────────────────────────────
        let travel_prep = {
            let mut shutdown_rx = shutdown_tx.subscribe();
            let db = db.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            tracing::info!("[WorkerPool] Travel-prep worker tick");
                            let _ = &db;
                        }
                        _ = shutdown_rx.recv() => {
                            tracing::info!("[WorkerPool] Travel-prep worker shutting down");
                            break;
                        }
                    }
                }
            })
        };

        // ── Memory consolidator worker ──────────────────────────────────
        let memory_consolidator = {
            let mut shutdown_rx = shutdown_tx.subscribe();
            let db = db.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(600));
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            tracing::info!("[WorkerPool] Memory-consolidator worker tick");
                            let _ = &db;
                        }
                        _ = shutdown_rx.recv() => {
                            tracing::info!("[WorkerPool] Memory-consolidator worker shutting down");
                            break;
                        }
                    }
                }
            })
        };

        // ── Collapse worker ─────────────────────────────────────
        let (collapse_tx, collapse_rx) = mpsc::channel::<String>(256);
        let collapse = {
            let db = db.clone();
            let llm_provider = llm_provider.clone();
            let collapse_model = _config.collapse_model.clone();

            let collapse_prompt = {
                // Read collapse_prompt from settings synchronously for now
                let mut collapse_prompt = "Resume el siguiente texto manteniendo la información clave, los datos importantes y el contexto necesario. Sé conciso.".to_string();

                // Spawn a separate task to read settings async and store the result
                let db_for_prompt = db.clone();
                let prompt_future = async move {
                    crate::db::repos::settings::SettingsRepo::get(&db_for_prompt, "collapse_prompt")
                        .await
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| "Resume el siguiente texto manteniendo la información clave, los datos importantes y el contexto necesario. Sé conciso.".to_string())
                };

                // We need to block on this because `start()` is not async
                // Use tokio::runtime::Handle to run the future on the current runtime
                if let Ok(handle) = tokio::runtime::Handle::try_current() {
                    collapse_prompt =
                        tokio::task::block_in_place(|| handle.block_on(prompt_future));
                }

                collapse_prompt
            };

            let mut shutdown_rx = shutdown_tx.subscribe();
            tokio::spawn(async move {
                let handle = crate::workers::collapse::CollapseWorker::start(
                    db,
                    llm_provider,
                    collapse_rx,
                    collapse_prompt,
                    collapse_model,
                );
                // Esperar shutdown o que el worker termine
                let _ = shutdown_rx.recv().await;
                handle.abort();
            })
        };

        Self {
            briefing: Some(briefing),
            conflict_detector: Some(conflict_detector),
            travel_prep: Some(travel_prep),
            memory_consolidator: Some(memory_consolidator),
            collapse: Some(collapse),
            collapse_tx: Some(collapse_tx),
            shutdown_tx: Some(shutdown_tx),
        }
    }

    /// Gracefully shut down all workers.
    ///
    /// Sends a shutdown signal through the broadcast channel and aborts all
    /// task handles.
    pub async fn shutdown(&mut self) {
        // Send shutdown signal to all workers
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        // Abort each handle
        if let Some(handle) = self.briefing.take() {
            handle.abort();
        }
        if let Some(handle) = self.conflict_detector.take() {
            handle.abort();
        }
        if let Some(handle) = self.travel_prep.take() {
            handle.abort();
        }
        if let Some(handle) = self.memory_consolidator.take() {
            handle.abort();
        }
        if let Some(handle) = self.collapse.take() {
            handle.abort();
        }
        self.collapse_tx.take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::db::repos::messages::MessagesRepo;
    use crate::db::schema::run_migrations;
    use crate::llm::provider::{ChatMessage, ChatRequest, ChatResponse, LLMError, TokenUsage};
    use async_trait::async_trait;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use sqlx::SqlitePool;
    use std::sync::{Arc, Mutex};

    /// Create a minimal [`DbPool`] with an in-memory database for tests.
    async fn test_db() -> DbPool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await
            .expect("Failed to create in-memory database for test");
        run_migrations(&pool)
            .await
            .expect("Failed to run migrations");
        pool
    }

    /// A mock LLM provider that records chat requests and returns canned responses.
    struct MockPoolLLM {
        pub calls: Arc<Mutex<Vec<ChatRequest>>>,
    }

    #[async_trait]
    impl crate::llm::provider::LLMProvider for MockPoolLLM {
        async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, LLMError> {
            self.calls.lock().unwrap().push(request);
            Ok(ChatResponse {
                message: ChatMessage {
                    role: "assistant".into(),
                    content: "Resumen del mensaje.".into(),
                    tool_calls: None,
                    tool_result: None,
                    tool_call_id: None,
                },
                usage: Some(TokenUsage {
                    prompt_tokens: 100,
                    completion_tokens: 50,
                }),
            })
        }

        async fn chat_stream(
            &self,
            _request: ChatRequest,
        ) -> Result<
            std::pin::Pin<
                Box<
                    dyn tokio_stream::Stream<
                            Item = Result<crate::llm::provider::StreamEvent, LLMError>,
                        > + Send,
                >,
            >,
            LLMError,
        > {
            unimplemented!("chat_stream not used in tests")
        }

        async fn embed(&self, _input: &str) -> Result<Vec<f32>, LLMError> {
            unimplemented!("embed not used in tests")
        }
    }

    fn test_llm_provider() -> Arc<dyn crate::llm::provider::LLMProvider> {
        Arc::new(MockPoolLLM {
            calls: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Create a [`Config`] with default values for testing.
    fn test_config() -> Config {
        Config {
            host: "0.0.0.0".into(),
            port: 3000,
            database_url: ":memory:".into(),
            log_level: "debug".into(),
            openrouter_api_key: None,
            openrouter_model: "anthropic/claude-sonnet-20241022".into(),
            openrouter_base_url: "https://openrouter.ai/api/v1".into(),
            ollama_base_url: "http://localhost:11434".into(),
            ollama_model: "llama3.2:3b".into(),
            auth_enabled: false,
            auth_issuer_url: "http://localhost:8080".into(),
            auth_client_id: String::new(),
            auth_client_secret: String::new(),
            auth_redirect_url: "http://localhost:3000/auth/callback".into(),
            jwt_secret: String::new(),
            openweather_api_key: None,
            google_places_api_key: None,
            brave_search_api_key: None,
            briefing_time: "08:15".into(),
            consolidation_time: "23:00".into(),
            travel_prep_days_before: 3,
            collapse_threshold_tokens: 2000,
            collapse_model: "mistralai/mistral-small".into(),
        }
    }

    /// WorkerPool::start() must not panic and should return a pool with all
    /// four worker handles set to `Some`.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_worker_pool_start() {
        let db = test_db().await;
        let config = test_config();
        let mut pool = WorkerPool::start(db, &config, test_llm_provider());

        assert!(pool.briefing.is_some(), "Briefing worker should be Some");
        assert!(
            pool.conflict_detector.is_some(),
            "Conflict detector worker should be Some"
        );
        assert!(
            pool.travel_prep.is_some(),
            "Travel prep worker should be Some"
        );
        assert!(
            pool.memory_consolidator.is_some(),
            "Memory consolidator worker should be Some"
        );
        assert!(pool.collapse.is_some(), "Collapse worker should be Some");
        assert!(
            pool.collapse_tx.is_some(),
            "Collapse channel sender should be Some"
        );
        assert!(pool.shutdown_tx.is_some(), "Shutdown sender should be Some");

        // Clean up to avoid lingering tasks
        pool.shutdown().await;
    }

    /// WorkerPool::shutdown() must cleanly abort all workers without panicking.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_worker_pool_shutdown() {
        let db = test_db().await;
        let config = test_config();
        let mut pool = WorkerPool::start(db, &config, test_llm_provider());

        // Give workers a moment to tick
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Shutdown should not panic
        pool.shutdown().await;

        // After shutdown, all handles should have been taken
        assert!(pool.briefing.is_none());
        assert!(pool.conflict_detector.is_none());
        assert!(pool.travel_prep.is_none());
        assert!(pool.memory_consolidator.is_none());
        assert!(pool.collapse.is_none());
        assert!(pool.collapse_tx.is_none());
        assert!(pool.shutdown_tx.is_none());
    }

    /// Given a WorkerPool started with a Config that specifies a collapse_model,
    /// when a long message's ID is sent through the collapse channel,
    /// then the real CollapseWorker must process it and set collapsed_content in the DB.
    ///
    /// This test uses real sqllite and sqlx async pool. It requires `SQLX_OFFLINE=true`
    /// or a running database to build queries; the in-memory pool avoids needing a server.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_pool_uses_real_collapse_worker() {
        // Create an in-memory DB with migrations and a long message
        let pool = test_db().await;

        let long_content = "x".repeat(8000);
        let msg = MessagesRepo::create(&pool, "user", &long_content, None, None, 2000, None)
            .await
            .unwrap();
        let msg_id = msg.id.clone();

        let config = test_config();
        let mut pool_workers = WorkerPool::start(pool.clone(), &config, test_llm_provider());

        // Send the message ID through the collapse channel
        if let Some(tx) = &pool_workers.collapse_tx {
            tx.send(msg_id.clone()).await.unwrap();
        } else {
            panic!("collape_tx should be Some");
        }

        // Give the worker time to process
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        // Verify the message was processed: collapsed_content should be set
        let processed = MessagesRepo::find_by_id(&pool, &msg_id)
            .await
            .unwrap()
            .expect("Message should exist");

        assert!(
            processed.collapsed_content.is_some(),
            "The real CollapseWorker should have set collapsed_content, \
             but the placeholder only logs — this test will FAIL (RED)"
        );

        pool_workers.shutdown().await;
    }
}
