use crate::config::Config;
use crate::db::repos::events::EventsRepo;
use crate::db::DbPool;
use crate::workers::briefing::BriefingWorker;
use crate::workers::conflict_detector::ConflictDetector;
use crate::workers::episodic_memory::{EpisodicMemoryConfig, EpisodicMemoryWorker};
use crate::workers::travel_prep::TravelPrepWorker;
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
    pub memory: Option<JoinHandle<()>>,
    pub memory_tx: Option<mpsc::Sender<()>>,
    pub shutdown_tx: Option<broadcast::Sender<()>>,
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
                            let worker = BriefingWorker::new(db.clone(), None);
                            match worker.generate().await {
                                Ok(briefing) => {
                                    tracing::info!("[BriefingWorker] Daily briefing:\n{}", briefing);
                                }
                                Err(e) => {
                                    tracing::error!("[BriefingWorker] Failed to generate briefing: {}", e);
                                }
                            }
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
                            // Get first available profile
                            let profile_id = match sqlx::query_scalar::<_, String>("SELECT id FROM profiles LIMIT 1")
                                .fetch_optional(&db)
                                .await
                            {
                                Ok(Some(id)) => id,
                                Ok(None) => {
                                    tracing::warn!("[ConflictDetector] No profiles found");
                                    return;
                                }
                                Err(e) => {
                                    tracing::error!("[ConflictDetector] Failed to query profile: {}", e);
                                    return;
                                }
                            };

                            let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                            let detector = ConflictDetector::new(db.clone());
                            match detector.check_date(&profile_id, &today).await {
                                Ok(alerts) => {
                                    if alerts.is_empty() {
                                        tracing::info!("[ConflictDetector] No conflicts found for today");
                                    } else {
                                        for alert in &alerts {
                                            tracing::info!(
                                                "[ConflictDetector] Conflict: {:?} between '{}' and '{}' (gap: {} min)",
                                                alert.severity, alert.event_a.title, alert.event_b.title, alert.gap_minutes
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("[ConflictDetector] Error: {}", e);
                                }
                            }
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
                            let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                            let three_days = (chrono::Utc::now() + chrono::Duration::days(3))
                                .format("%Y-%m-%d")
                                .to_string();

                            let profile_id = match sqlx::query_scalar::<_, String>("SELECT id FROM profiles LIMIT 1")
                                .fetch_optional(&db)
                                .await
                            {
                                Ok(Some(id)) => id,
                                Ok(None) => {
                                    tracing::warn!("[TravelPrep] No profiles found");
                                    return;
                                }
                                Err(e) => {
                                    tracing::error!("[TravelPrep] Failed to query profile: {}", e);
                                    return;
                                }
                            };

                            match EventsRepo::list_by_date_range(&db, &profile_id, &today, &three_days).await {
                                Ok(events) => {
                                    let trip_events: Vec<_> = events.into_iter()
                                        .filter(|e| e.location.as_deref().map(|l| !l.is_empty()).unwrap_or(false))
                                        .collect();

                                    if trip_events.is_empty() {
                                        tracing::info!("[TravelPrep] No upcoming trips found in next 3 days");
                                    } else {
                                        let prep_worker = TravelPrepWorker::new(db.clone(), 3);
                                        for event in &trip_events {
                                            let location = event.location.as_deref().unwrap_or("");
                                            match prep_worker.prepare_for_trip(&event.title, location) {
                                                Ok(prep) => {
                                                    tracing::info!("[TravelPrep] Trip preparation for '{}':\n{}", event.title, prep);
                                                }
                                                Err(e) => {
                                                    tracing::error!("[TravelPrep] Error preparing trip '{}': {}", event.title, e);
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("[TravelPrep] Error querying events: {}", e);
                                }
                            }
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

        // ── Episodic memory worker ─────────────────────────────────────
        let (memory_tx, memory_rx) = mpsc::channel::<()>(256);
        let memory = {
            let shutdown_rx = shutdown_tx.subscribe();
            let db = db.clone();
            let llm_provider = llm_provider.clone();
            let memory_config = EpisodicMemoryConfig {
                batch_tokens: _config.memory_batch_tokens,
                inactivity_minutes: _config.memory_inactivity_minutes as i64,
                overlap: _config.memory_overlap,
                poll_interval_minutes: _config.memory_poll_interval_minutes,
                model: _config.memory_model.clone(),
            };
            EpisodicMemoryWorker::start(db, llm_provider, memory_rx, shutdown_rx, memory_config)
        };

        Self {
            briefing: Some(briefing),
            conflict_detector: Some(conflict_detector),
            travel_prep: Some(travel_prep),
            memory_consolidator: Some(memory_consolidator),
            collapse: Some(collapse),
            collapse_tx: Some(collapse_tx),
            memory: Some(memory),
            memory_tx: Some(memory_tx),
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
        if let Some(handle) = self.memory.take() {
            handle.abort();
        }
        self.memory_tx.take();
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
            memory_batch_tokens: 2000,
            memory_inactivity_minutes: 30,
            memory_overlap: 2,
            memory_poll_interval_minutes: 30,
            memory_model: "mistralai/mistral-small".into(),
            rag_budget_tokens: 2000,
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
        assert!(
            pool.memory.is_some(),
            "Memory worker should be Some — RED: currently None, will pass after GREEN wiring"
        );
        assert!(pool.memory_tx.is_some(), "Memory channel sender should be Some — RED: currently None, will pass after GREEN wiring");
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
        assert!(pool.memory.is_none());
        assert!(pool.memory_tx.is_none());
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
        let msg = MessagesRepo::create(&pool, "user", &long_content, None, None, None, 2000, None)
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

    /// Sanity check: `shutdown_tx` is publicly accessible and is `Some`
    /// immediately after `WorkerPool::start()`.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_worker_pool_shutdown_tx_accessible() {
        let db = test_db().await;
        let config = test_config();
        let mut pool = WorkerPool::start(db, &config, test_llm_provider());

        assert!(
            pool.shutdown_tx.is_some(),
            "shutdown_tx should be Some after WorkerPool::start()"
        );

        pool.shutdown().await;
    }

    // ═══════════════════════════════════════════════════════════════════
    // RED tests — prove placeholders do NOT call real worker impls
    // ═══════════════════════════════════════════════════════════════════
    //
    // These tests will FAIL to compile (RED) because `BriefingWorker`,
    // `ConflictDetector`, and `TravelPrepWorker` are not imported anywhere
    // in `pool.rs`. The GREEN phase will add the imports AND wire the real
    // workers into the tick handlers.

    /// [RED] Prove BriefingWorker is not wired into the pool.
    ///
    /// References `BriefingWorker::new()` and `generate()` directly.
    /// This will fail to compile because `BriefingWorker` is not imported
    /// in pool.rs — the placeholder only logs "tick".
    #[tokio::test(flavor = "multi_thread")]
    async fn test_pool_briefing_worker_accessible() {
        let db = test_db().await;

        // Seed a profile so BriefingWorker::generate() can query the DB
        sqlx::query(
            "INSERT INTO profiles (id, name, preferences) VALUES ('profile-id', 'Test', '{}')",
        )
        .execute(&db)
        .await
        .expect("Failed to seed profile");

        // This line will fail to compile: `BriefingWorker` is not in scope.
        // The pool's briefing placeholder calls `let _ = &db;` instead of
        // instantiating BriefingWorker and calling generate().
        let worker = BriefingWorker::new(db, None);
        let result = worker.generate().await;
        assert!(result.is_ok(), "BriefingWorker::generate() should succeed");
    }

    /// [RED] Prove ConflictDetector is not wired into the pool.
    ///
    /// References `ConflictDetector::new()` and `check_date()` directly.
    /// Will fail to compile because `ConflictDetector` is not imported.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_pool_conflict_detector_accessible() {
        let db = test_db().await;

        // Seed a profile so ConflictDetector::check_date() can query events
        sqlx::query(
            "INSERT INTO profiles (id, name, preferences, created_at, updated_at)
             VALUES ('profile-1', 'Test', '{}', '2026-09-24T00:00:00Z', '2026-09-24T00:00:00Z')",
        )
        .execute(&db)
        .await
        .expect("Failed to seed profile");

        // Insert two overlapping events so check_date() returns a Critical alert
        sqlx::query(
            "INSERT INTO events (id, profile_id, title, description, start_time, end_time,
                                 location, scope, category, all_day, rrule,
                                 reminder_minutes_before, created_at, updated_at)
             VALUES ('evt-a', 'profile-1', 'Event A', '', '2026-09-24T09:00:00', '2026-09-24T10:30:00',
                     NULL, 'personal', 'default', 0, NULL, NULL,
                     '2026-09-24T00:00:00Z', '2026-09-24T00:00:00Z')",
        )
        .execute(&db)
        .await
        .expect("Failed to seed event A");

        sqlx::query(
            "INSERT INTO events (id, profile_id, title, description, start_time, end_time,
                                 location, scope, category, all_day, rrule,
                                 reminder_minutes_before, created_at, updated_at)
             VALUES ('evt-b', 'profile-1', 'Event B', '', '2026-09-24T10:00:00', '2026-09-24T11:00:00',
                     NULL, 'personal', 'default', 0, NULL, NULL,
                     '2026-09-24T00:00:00Z', '2026-09-24T00:00:00Z')",
        )
        .execute(&db)
        .await
        .expect("Failed to seed event B");

        // This line will fail to compile: `ConflictDetector` is not in scope.
        // The pool's conflict-detector placeholder calls `let _ = &db;` instead.
        let detector = ConflictDetector::new(db);
        let alerts = detector.check_date("profile-1", "2026-09-24").await;
        assert!(
            alerts.is_ok(),
            "ConflictDetector::check_date() should succeed"
        );
        let alerts = alerts.unwrap();
        assert!(
            !alerts.is_empty(),
            "Overlapping events should produce alerts"
        );
    }

    /// [RED] Prove TravelPrepWorker is not wired into the pool.
    ///
    /// References `TravelPrepWorker::new()` and `find_upcoming_trips()` directly.
    /// Will fail to compile because `TravelPrepWorker` is not imported.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_pool_travel_prep_accessible() {
        let db = test_db().await;
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

        // Seed a profile
        sqlx::query(
            "INSERT INTO profiles (id, name, preferences) VALUES ('profile-1', 'Test', '{}')",
        )
        .execute(&db)
        .await
        .expect("Failed to seed profile");

        // Insert an event WITH a location in the next 3 days — the real worker
        // should find it via find_upcoming_trips().
        sqlx::query(
            "INSERT INTO events (id, profile_id, title, description, start_time, end_time,
                                 location, scope, category, all_day, rrule,
                                 reminder_minutes_before, created_at, updated_at)
             VALUES ($1, 'profile-1', 'Viaje a Paris', '', $2 || 'T10:00:00Z', $2 || 'T11:00:00Z',
                     'Paris, Francia', 'shared', 'default', 0, NULL, NULL,
                     $2 || 'T00:00:00Z', $2 || 'T00:00:00Z')",
        )
        .bind("evt-trip-1")
        .bind(&today)
        .execute(&db)
        .await
        .expect("Failed to seed event with location");

        // This line will fail to compile: `TravelPrepWorker` is not in scope.
        // The pool's travel-prep placeholder calls `let _ = &db;` instead.
        let worker = TravelPrepWorker::new(db, 3);
        let trips = worker.find_upcoming_trips("profile-1").await;
        assert!(trips.is_ok(), "find_upcoming_trips() should succeed");
        let trips = trips.unwrap();
        assert!(!trips.is_empty(), "Should find at least one trip");
    }
}
