use crate::config::Config;
use crate::db::DbPool;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;

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
    pub fn start(db: DbPool, _config: &Config) -> Self {
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

        // ── Collapse worker (placeholder) ────────────────────────────────
        // The collapse worker listens for message IDs on an mpsc channel and
        // sends them to an LLM for summarisation. For now this is a minimal
        // placeholder that logs received IDs without actually calling an LLM.
        let (collapse_tx, collapse_rx) = mpsc::channel::<String>(256);
        let collapse = {
            let mut shutdown_rx = shutdown_tx.subscribe();
            tokio::spawn(async move {
                let mut collapse_rx = collapse_rx;
                loop {
                    tokio::select! {
                        Some(msg_id) = collapse_rx.recv() => {
                            tracing::info!("[WorkerPool] Collapse worker received message: {}", msg_id);
                        }
                        _ = shutdown_rx.recv() => {
                            tracing::info!("[WorkerPool] Collapse worker shutting down");
                            break;
                        }
                    }
                }
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
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};

    /// Create a minimal [`DbPool`] with an in-memory database for tests.
    fn test_db() -> DbPool {
        let conn =
            Connection::open_in_memory().expect("Failed to create in-memory database for test");
        Arc::new(Mutex::new(conn))
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
            briefing_time: "08:15".into(),
            consolidation_time: "23:00".into(),
            travel_prep_days_before: 3,
            collapse_threshold_tokens: 2000,
            collapse_model: "mistralai/mistral-small".into(),
        }
    }

    /// WorkerPool::start() must not panic and should return a pool with all
    /// four worker handles set to `Some`.
    #[tokio::test]
    async fn test_worker_pool_start() {
        let db = test_db();
        let config = test_config();
        let mut pool = WorkerPool::start(db, &config);

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
    #[tokio::test]
    async fn test_worker_pool_shutdown() {
        let db = test_db();
        let config = test_config();
        let mut pool = WorkerPool::start(db, &config);

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
}
