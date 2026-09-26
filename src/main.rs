use alfred::config::Config;
use alfred::{app_with_state, AppState};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from environment variables
    let config = Config::from_env();

    // Initialize tracing/logging with level from config
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level)),
        )
        .init();

    tracing::info!(model = %config.openrouter_model, "Configuration loaded");

    // Build full application state (database, orchestrator, tools, guardrails, auth)
    let state = AppState::new_with_orchestrator(&config.database_url).await?;

    // Spawn the stats cleanup worker (runs hourly, purges old llm_requests)
    let pool = state.db.clone();
    tokio::spawn(alfred::workers::stats_cleanup::run_cleanup_worker(pool));

    // Build the application router
    let router = app_with_state(state);

    // Bind and serve on configured host:port
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("Alfred server listening on {addr}");

    axum::serve(listener, router).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use alfred::AppState;
    use serial_test::serial;
    use std::env;

    /// Verify that the production initialisation path
    /// (`AppState::new_with_orchestrator`) correctly wires up every
    /// optional component (orchestrator, tool_registry, guardrails,
    /// auth_config) to `Some` instead of leaving them as `None`.
    ///
    /// **Red-phase rationale:**
    ///
    /// The current `main()` manually constructs `AppState` with every
    /// optional field hard-coded to `None`. It does **not** call
    /// `Config::from_env()` or `AppState::new_with_orchestrator()`.
    ///
    /// This test encodes the *desired* contract: when the production
    /// entry point is used, all subsystems must be wired up. Once
    /// `main()` is refactored to use `Config::from_env()` +
    /// `AppState::new_with_orchestrator()`, this assertion will hold.
    #[tokio::test]
    #[serial]
    async fn test_production_state_initialization() {
        // -----------------------------------------------------------------
        // Arrange: set the minimum environment variables required by
        //          `AppState::new_with_orchestrator`.
        // -----------------------------------------------------------------
        env::set_var("OPENROUTER_API_KEY", "sk-test-key-for-unit-test");
        env::set_var("AUTH_ENABLED", "true");
        env::set_var("AUTH_CLIENT_ID", "test-client");
        env::set_var("AUTH_CLIENT_SECRET", "test-secret");
        env::set_var("JWT_SECRET", "test-jwt-secret");

        let tmp_dir = env::temp_dir();
        let db_filename = "alfred_test_production_state.db";
        let db_path = tmp_dir.join(db_filename);
        // Remove any stale database from a previous (failed) run
        let _ = std::fs::remove_file(&db_path);

        // -----------------------------------------------------------------
        // Act: use the SAME entry point that main() SHOULD call
        // -----------------------------------------------------------------
        let state = AppState::new_with_orchestrator(
            db_path.to_str().expect("temp path must be valid UTF-8"),
        )
        .await
        .expect("new_with_orchestrator should succeed with minimal env vars");

        // -----------------------------------------------------------------
        // Assert: every component is wired up
        // -----------------------------------------------------------------
        assert!(
            state.orchestrator.is_some(),
            "orchestrator is None but should be Some — \
             main() currently hard-codes it to None"
        );
        assert!(
            state.tool_registry.is_some(),
            "tool_registry is None but should be Some — \
             main() currently hard-codes it to None"
        );
        assert!(
            state.guardrails.is_some(),
            "guardrails is None but should be Some — \
             main() currently hard-codes it to None"
        );
        assert!(
            state.auth_config.is_some(),
            "auth_config is None but should be Some — \
             main() currently hard-codes it to None"
        );

        // Additional verification on auth config values
        let auth = state.auth_config.as_ref().unwrap();
        assert!(auth.enabled, "auth should be enabled per AUTH_ENABLED=true");
        assert_eq!(auth.client_id, "test-client");

        // -----------------------------------------------------------------
        // Teardown: remove the temporary database + WAL/SHM artifacts
        // -----------------------------------------------------------------
        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_file(tmp_dir.join("alfred_test_production_state.db-wal"));
        let _ = std::fs::remove_file(tmp_dir.join("alfred_test_production_state.db-shm"));

        // Clean up environment variables so they don't leak into other tests
        env::remove_var("OPENROUTER_API_KEY");
        env::remove_var("AUTH_ENABLED");
        env::remove_var("AUTH_CLIENT_ID");
        env::remove_var("AUTH_CLIENT_SECRET");
        env::remove_var("JWT_SECRET");
    }
}
