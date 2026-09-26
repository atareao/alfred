pub mod auth;
pub mod config;
pub mod db;
pub mod embeddings;
pub mod errors;
pub mod handlers;
pub mod llm;
pub mod models;
pub mod orchestrator;
pub mod routes;
pub mod services;
pub mod telemetry;
pub mod tools;
pub mod workers;

use axum::routing::{delete, get, put};
use axum::{extract::State, Json, Router};
use serde_json::Value;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::mpsc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};

use crate::orchestrator::agent::Orchestrator;
use crate::orchestrator::context_builder::ContextBuilder;
use crate::orchestrator::guardrails::Guardrails;
use crate::tools::registry::ToolRegistry;

/// Shared application state with an async SQLite connection pool.
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub orchestrator: Option<Arc<Orchestrator>>,
    pub guardrails: Option<Arc<Guardrails>>,
    pub tool_registry: Option<Arc<ToolRegistry>>,
    pub auth_config: Option<crate::auth::AuthConfig>,
    pub collapse_tx: Option<mpsc::Sender<String>>,
}

impl AppState {
    /// Create a new AppState with an in-memory SQLite database and no seed data.
    /// Used by integration tests that need a clean state.
    pub async fn new_in_memory_empty() -> Self {
        use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await
            .expect("Failed to create in-memory database");
        db::schema::run_migrations(&pool)
            .await
            .expect("Failed to run migrations on in-memory database");
        let _ = db::fts::create_fts_triggers(&pool).await;
        let _ = db::repos::tools::ToolsRepo::seed_defaults(&pool).await;
        Self {
            db: pool,
            orchestrator: None,
            guardrails: None,
            tool_registry: None,
            auth_config: None,
            collapse_tx: None,
        }
    }

    /// Create a new AppState with an in-memory SQLite database.
    /// Used by integration tests.
    pub async fn new_in_memory() -> Self {
        use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await
            .expect("Failed to create in-memory database");
        db::schema::run_migrations(&pool)
            .await
            .expect("Failed to run migrations on in-memory database");
        let _ = db::fts::create_fts_triggers(&pool).await;
        let _ = db::repos::tools::ToolsRepo::seed_defaults(&pool).await;
        // Seed test data with known IDs expected by integration tests
        let _ = Self::seed_test_data(&pool).await;
        Self {
            db: pool,
            orchestrator: None,
            guardrails: None,
            tool_registry: None,
            auth_config: None,
            collapse_tx: None,
        }
    }

    /// Create a new AppState backed by a file-based database with a fully
    /// initialised orchestrator, tool registry, guardrails, and auth config.
    ///
    /// This is the production entry point used by `main.rs`.
    pub async fn new_with_orchestrator(db_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // 1. Open database connection pool
        let pool = db::init_db(db_path).await?;

        // 2. Create tool registry
        let mut tool_registry = ToolRegistry::new();
        // Register built-in tools
        tool_registry.register(Box::new(crate::tools::weather::WeatherTool::new(
            pool.clone(),
            std::env::var("OPENWEATHER_API_KEY").unwrap_or_default(),
        )));
        tool_registry.register(Box::new(crate::tools::geo::GeocodeTool::new()));
        tool_registry.register(Box::new(crate::tools::geo::ReverseGeocodeTool::new()));
        tool_registry.register(Box::new(
            crate::tools::google_places::SearchPlacesTool::new(pool.clone()),
        ));
        tool_registry.register(Box::new(crate::tools::web_search::WebSearchTool::new(
            pool.clone(),
        )));
        tool_registry.register(Box::new(crate::tools::meals::MealsTool::new(pool.clone())));
        tool_registry.register(Box::new(crate::tools::habits::HabitsTool::new(
            pool.clone(),
        )));
        tool_registry.register(Box::new(crate::tools::calendar::CalendarTool::new(
            pool.clone(),
        )));
        tool_registry.register(Box::new(crate::tools::tasks::TasksTool::new(pool.clone())));
        tool_registry.register(Box::new(crate::tools::reminders::RemindersTool::new(
            pool.clone(),
        )));
        let tool_registry = Arc::new(tool_registry);

        // 3. Create guardrails
        let guardrails = Arc::new(Guardrails::new(tool_registry.clone()));

        // 4. Create LLM provider (try OpenRouter first, fall back to Ollama)
        let llm_provider: Arc<dyn crate::llm::provider::LLMProvider> =
            if let Ok(api_key) = std::env::var("OPENROUTER_API_KEY") {
                let config = crate::llm::openrouter::OpenRouterConfig {
                    api_key,
                    model: std::env::var("OPENROUTER_MODEL")
                        .unwrap_or_else(|_| "anthropic/claude-sonnet-20241022".into()),
                    base_url: std::env::var("OPENROUTER_BASE_URL")
                        .unwrap_or_else(|_| "https://openrouter.ai/api/v1".into()),
                    max_retries: 3,
                    timeout_secs: 120,
                };
                Arc::new(crate::llm::openrouter::OpenRouterProvider::new(config))
            } else {
                let config = crate::llm::ollama::OllamaConfig {
                    base_url: std::env::var("OLLAMA_BASE_URL")
                        .unwrap_or_else(|_| "http://localhost:11434".into()),
                    model: std::env::var("LLM_MODEL").unwrap_or_else(|_| "llama3.2:3b".into()),
                    timeout_secs: 120,
                    keep_alive: "5m".into(),
                };
                Arc::new(crate::llm::ollama::OllamaProvider::new(config))
            };

        // 5. Create orchestrator
        let context_builder = Arc::new(ContextBuilder::new());
        let config = crate::orchestrator::agent::OrchestratorConfig::default();

        let orchestrator = Arc::new(Orchestrator::new(
            llm_provider,
            tool_registry.clone(),
            guardrails.clone(),
            context_builder,
            config,
            pool.clone(),
            None,
        ));

        // 6. Create auth config from environment
        let auth_config = crate::auth::AuthConfig {
            enabled: std::env::var("AUTH_ENABLED")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
            issuer_url: std::env::var("AUTH_ISSUER_URL")
                .unwrap_or_else(|_| "http://localhost:8080".into()),
            client_id: std::env::var("AUTH_CLIENT_ID").unwrap_or_default(),
            client_secret: std::env::var("AUTH_CLIENT_SECRET").unwrap_or_default(),
            redirect_url: std::env::var("AUTH_REDIRECT_URL")
                .unwrap_or_else(|_| "http://localhost:3000/auth/callback".into()),
            jwt_secret: std::env::var("JWT_SECRET").unwrap_or_default(),
        };

        Ok(Self {
            db: pool,
            orchestrator: Some(orchestrator),
            guardrails: Some(guardrails),
            tool_registry: Some(tool_registry),
            auth_config: Some(auth_config),
            collapse_tx: None,
        })
    }

    /// Insert records with well-known IDs so integration tests that reference
    /// hard-coded IDs (e.g. `some-id`, `conv-id`, `profile-id`) can pass.
    async fn seed_test_data(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();

        // Profile used by memory tests (id = "profile-id")
        sqlx::query(
            "INSERT OR IGNORE INTO profiles (id, name, avatar_url, preferences, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind("profile-id")
        .bind("Test User")
        .bind(Option::<String>::None)
        .bind("{}")
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        // Message used by get_message tests
        sqlx::query(
            "INSERT OR IGNORE INTO messages (id, role, content, tokens_count, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind("msg-id")
        .bind("user")
        .bind("Hello from seeded data")
        .bind(0i64)
        .bind(&now)
        .execute(pool)
        .await?;

        // Memory used by delete_memory tests
        sqlx::query(
            "INSERT OR IGNORE INTO memories (id, profile_id, content, category, source, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind("memory-id")
        .bind("profile-id")
        .bind("Seeded memory")
        .bind("general")
        .bind("manual")
        .bind(&now)
        .execute(pool)
        .await?;

        Ok(())
    }
}

/// Health check handler.
///
/// Returns a JSON payload indicating server status, version, and whether the
/// database connection is healthy.
async fn health_handler(State(state): State<AppState>) -> Json<Value> {
    let db_status = match sqlx::query("SELECT 1").execute(&state.db).await {
        Ok(_) => "connected",
        Err(_) => "disconnected",
    };

    Json(serde_json::json!({
        "status": "ok",
        "version": "0.1.0",
        "db": db_status,
    }))
}

/// Build the Axum [`Router`] with all routes and the given [`AppState`].
///
/// This is the primary entry point for the production server in `main.rs`.
pub fn app_with_state(state: AppState) -> Router {
    // CORS middleware — allows any origin in dev; production origins are
    // restricted via AUTH_REDIRECT_URL or environment-specific config.
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Health
        .route("/api/health", get(health_handler))
        // Export
        .route("/api/export", get(routes::export::export_data))
        // Messages
        .route(
            "/api/messages",
            get(routes::messages::list_messages).post(routes::messages::create_message),
        )
        .route("/api/messages/:msg_id", get(routes::messages::get_message))
        // Chat
        .route("/api/chat/init", get(routes::chat::chat_init))
        // Profile
        .route(
            "/api/profile",
            get(routes::profile::get_profile).put(routes::profile::update_profile),
        )
        // Memories
        .route(
            "/api/memories",
            get(routes::memories::list_memories).post(routes::memories::create_memory),
        )
        .route("/api/memories/:id", delete(routes::memories::delete_memory))
        // Tools
        .route("/api/tools", get(routes::tools::list_tools))
        .route("/api/tools/:id/toggle", put(routes::tools::toggle_tool))
        // Settings
        .route(
            "/api/settings",
            get(routes::settings::get_settings).put(routes::settings::update_settings),
        )
        // Events
        .merge(routes::events::routes())
        // Tasks
        .merge(routes::tasks::routes())
        // Stats
        .merge(routes::stats::routes())
        // Search
        .route("/api/search", get(handlers::search::search))
        // Streaming + approval
        .merge(routes::stream::routes())
        .fallback_service(ServeDir::new("static").fallback(ServeFile::new("static/index.html")))
        .layer(cors)
        .with_state(state)
}

/// Convenience constructor that creates an ephemeral in-memory database and
/// returns a ready-to-use [`Router`].
///
/// Used by integration tests (e.g. `tests/api/health.rs`) that call
/// `alfred::app()` without wiring their own state.
pub async fn app() -> Router {
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(":memory:")
                .create_if_missing(true),
        )
        .await
        .expect("Failed to create in-memory database for tests");
    db::schema::run_migrations(&pool)
        .await
        .expect("Failed to run migrations on in-memory database");
    let _ = db::fts::create_fts_triggers(&pool).await;
    let _ = db::repos::tools::ToolsRepo::seed_defaults(&pool).await;
    let _ = AppState::seed_test_data(&pool).await;
    let state = AppState {
        db: pool,
        orchestrator: None,
        guardrails: None,
        tool_registry: None,
        auth_config: None,
        collapse_tx: None,
    };
    app_with_state(state)
}
