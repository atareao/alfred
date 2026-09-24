use std::env;

/// Centralized configuration for Alfred.
///
/// All environment variables are read once via [`Config::from_env`] and
/// exposed as typed fields with sensible defaults.
#[derive(Debug, Clone)]
pub struct Config {
    // Server
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub log_level: String,

    // LLM
    pub openrouter_api_key: Option<String>,
    pub openrouter_model: String,
    pub openrouter_base_url: String,
    pub ollama_base_url: String,
    pub ollama_model: String,

    // Auth
    pub auth_enabled: bool,
    pub auth_issuer_url: String,
    pub auth_client_id: String,
    pub auth_client_secret: String,
    pub auth_redirect_url: String,
    pub jwt_secret: String,

    // Weather
    pub openweather_api_key: Option<String>,

    // Workers
    pub briefing_time: String,
    pub consolidation_time: String,
    pub travel_prep_days_before: u32,
    // Message collapse threshold
    pub collapse_threshold_tokens: usize,
    // Model used for collapse/summary
    pub collapse_model: String,
}

impl Config {
    /// Build a [`Config`] from environment variables, applying sensible
    /// defaults whenever a variable is not set or cannot be parsed.
    pub fn from_env() -> Self {
        Self {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: env::var("PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3000),
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| "alfred.db".into()),
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".into()),

            openrouter_api_key: env::var("OPENROUTER_API_KEY").ok(),
            openrouter_model: env::var("OPENROUTER_MODEL")
                .unwrap_or_else(|_| "anthropic/claude-sonnet-20241022".into()),
            openrouter_base_url: env::var("OPENROUTER_BASE_URL")
                .unwrap_or_else(|_| "https://openrouter.ai/api/v1".into()),
            ollama_base_url: env::var("OLLAMA_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:11434".into()),
            ollama_model: env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2:3b".into()),

            auth_enabled: env::var("AUTH_ENABLED")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
            auth_issuer_url: env::var("AUTH_ISSUER_URL")
                .unwrap_or_else(|_| "http://localhost:8080".into()),
            auth_client_id: env::var("AUTH_CLIENT_ID").unwrap_or_default(),
            auth_client_secret: env::var("AUTH_CLIENT_SECRET").unwrap_or_default(),
            auth_redirect_url: env::var("AUTH_REDIRECT_URL")
                .unwrap_or_else(|_| "http://localhost:3000/auth/callback".into()),
            jwt_secret: env::var("JWT_SECRET").unwrap_or_default(),

            openweather_api_key: env::var("OPENWEATHER_API_KEY").ok(),

            briefing_time: env::var("BRIEFING_TIME").unwrap_or_else(|_| "08:15".into()),
            consolidation_time: env::var("CONSOLIDATION_TIME").unwrap_or_else(|_| "23:00".into()),
            travel_prep_days_before: env::var("TRAVEL_PREP_DAYS_BEFORE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3),
            collapse_threshold_tokens: env::var("COLLAPSE_THRESHOLD_TOKENS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2000),
            collapse_model: env::var("COLLAPSE_MODEL")
                .unwrap_or_else(|_| "mistralai/mistral-small".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    /// When no environment variables are set, [`Config::from_env`] must
    /// return the documented default values.
    #[test]
    #[serial]
    fn test_config_defaults() {
        // Unset any variables that may be set in the test environment
        // so we always get defaults.
        for var in [
            "HOST",
            "PORT",
            "DATABASE_URL",
            "LOG_LEVEL",
            "OPENROUTER_API_KEY",
            "OPENROUTER_MODEL",
            "OPENROUTER_BASE_URL",
            "OLLAMA_BASE_URL",
            "OLLAMA_MODEL",
            "AUTH_ENABLED",
            "AUTH_ISSUER_URL",
            "AUTH_CLIENT_ID",
            "AUTH_CLIENT_SECRET",
            "AUTH_REDIRECT_URL",
            "JWT_SECRET",
            "OPENWEATHER_API_KEY",
            "BRIEFING_TIME",
            "CONSOLIDATION_TIME",
            "TRAVEL_PREP_DAYS_BEFORE",
            "COLLAPSE_THRESHOLD_TOKENS",
            "COLLAPSE_MODEL",
        ] {
            env::remove_var(var);
        }

        let cfg = Config::from_env();

        assert_eq!(cfg.host, "0.0.0.0");
        assert_eq!(cfg.port, 3000);
        assert_eq!(cfg.database_url, "alfred.db");
        assert_eq!(cfg.log_level, "info");

        assert!(cfg.openrouter_api_key.is_none());
        assert_eq!(cfg.openrouter_model, "anthropic/claude-sonnet-20241022");
        assert_eq!(cfg.openrouter_base_url, "https://openrouter.ai/api/v1");
        assert_eq!(cfg.ollama_base_url, "http://localhost:11434");
        assert_eq!(cfg.ollama_model, "llama3.2:3b");

        assert!(!cfg.auth_enabled);
        assert_eq!(cfg.auth_issuer_url, "http://localhost:8080");
        assert_eq!(cfg.auth_client_id, "");
        assert_eq!(cfg.auth_client_secret, "");
        assert_eq!(cfg.auth_redirect_url, "http://localhost:3000/auth/callback");
        assert_eq!(cfg.jwt_secret, "");

        assert!(cfg.openweather_api_key.is_none());

        assert_eq!(cfg.briefing_time, "08:15");
        assert_eq!(cfg.consolidation_time, "23:00");
        assert_eq!(cfg.travel_prep_days_before, 3);
        assert_eq!(cfg.collapse_threshold_tokens, 2000);
        assert_eq!(cfg.collapse_model, "mistralai/mistral-small");
    }

    /// When environment variables are set, [`Config::from_env`] must pick
    /// them up correctly, including type parsing and optional handling.
    #[test]
    #[serial]
    fn test_config_custom_values() {
        // Set custom values
        env::set_var("HOST", "127.0.0.1");
        env::set_var("PORT", "9090");
        env::set_var("DATABASE_URL", "custom.db");
        env::set_var("LOG_LEVEL", "debug");
        env::set_var("OPENROUTER_API_KEY", "sk-or-v1-test-key");
        env::set_var("OPENROUTER_MODEL", "anthropic/claude-opus-20240229");
        env::set_var("OPENROUTER_BASE_URL", "https://custom.openrouter.ai/v1");
        env::set_var("OLLAMA_BASE_URL", "http://ollama.local:11434");
        env::set_var("OLLAMA_MODEL", "mistral:7b");
        env::set_var("AUTH_ENABLED", "true");
        env::set_var("AUTH_ISSUER_URL", "https://auth.example.com");
        env::set_var("AUTH_CLIENT_ID", "my-client");
        env::set_var("AUTH_CLIENT_SECRET", "my-secret");
        env::set_var("AUTH_REDIRECT_URL", "https://app.example.com/callback");
        env::set_var("JWT_SECRET", "super-secret-key");
        env::set_var("OPENWEATHER_API_KEY", "weather-key-123");
        env::set_var("BRIEFING_TIME", "07:00");
        env::set_var("CONSOLIDATION_TIME", "22:30");
        env::set_var("TRAVEL_PREP_DAYS_BEFORE", "5");
        env::set_var("COLLAPSE_THRESHOLD_TOKENS", "500");
        env::set_var("COLLAPSE_MODEL", "google/gemini-2.0-flash-lite");

        let cfg = Config::from_env();

        assert_eq!(cfg.host, "127.0.0.1");
        assert_eq!(cfg.port, 9090);
        assert_eq!(cfg.database_url, "custom.db");
        assert_eq!(cfg.log_level, "debug");

        assert_eq!(cfg.openrouter_api_key.as_deref(), Some("sk-or-v1-test-key"));
        assert_eq!(cfg.openrouter_model, "anthropic/claude-opus-20240229");
        assert_eq!(cfg.openrouter_base_url, "https://custom.openrouter.ai/v1");
        assert_eq!(cfg.ollama_base_url, "http://ollama.local:11434");
        assert_eq!(cfg.ollama_model, "mistral:7b");

        assert!(cfg.auth_enabled);
        assert_eq!(cfg.auth_issuer_url, "https://auth.example.com");
        assert_eq!(cfg.auth_client_id, "my-client");
        assert_eq!(cfg.auth_client_secret, "my-secret");
        assert_eq!(cfg.auth_redirect_url, "https://app.example.com/callback");
        assert_eq!(cfg.jwt_secret, "super-secret-key");

        assert_eq!(cfg.openweather_api_key.as_deref(), Some("weather-key-123"));

        assert_eq!(cfg.briefing_time, "07:00");
        assert_eq!(cfg.consolidation_time, "22:30");
        assert_eq!(cfg.travel_prep_days_before, 5);
        assert_eq!(cfg.collapse_threshold_tokens, 500);
        assert_eq!(cfg.collapse_model, "google/gemini-2.0-flash-lite");

        // Clean up to avoid polluting other tests
        for var in [
            "HOST",
            "PORT",
            "DATABASE_URL",
            "LOG_LEVEL",
            "OPENROUTER_API_KEY",
            "OPENROUTER_MODEL",
            "OPENROUTER_BASE_URL",
            "OLLAMA_BASE_URL",
            "OLLAMA_MODEL",
            "AUTH_ENABLED",
            "AUTH_ISSUER_URL",
            "AUTH_CLIENT_ID",
            "AUTH_CLIENT_SECRET",
            "AUTH_REDIRECT_URL",
            "JWT_SECRET",
            "OPENWEATHER_API_KEY",
            "BRIEFING_TIME",
            "CONSOLIDATION_TIME",
            "TRAVEL_PREP_DAYS_BEFORE",
            "COLLAPSE_THRESHOLD_TOKENS",
            "COLLAPSE_MODEL",
        ] {
            env::remove_var(var);
        }
    }

    /// When COLLAPSE_THRESHOLD_TOKENS is not set, the default must be 2000.
    #[test]
    #[serial]
    fn test_config_collapse_threshold_default() {
        for var in [
            "HOST",
            "PORT",
            "DATABASE_URL",
            "LOG_LEVEL",
            "OPENROUTER_API_KEY",
            "OPENROUTER_MODEL",
            "OPENROUTER_BASE_URL",
            "OLLAMA_BASE_URL",
            "OLLAMA_MODEL",
            "AUTH_ENABLED",
            "AUTH_ISSUER_URL",
            "AUTH_CLIENT_ID",
            "AUTH_CLIENT_SECRET",
            "AUTH_REDIRECT_URL",
            "JWT_SECRET",
            "OPENWEATHER_API_KEY",
            "BRIEFING_TIME",
            "CONSOLIDATION_TIME",
            "TRAVEL_PREP_DAYS_BEFORE",
            "COLLAPSE_THRESHOLD_TOKENS",
            "COLLAPSE_MODEL",
        ] {
            env::remove_var(var);
        }

        let cfg = Config::from_env();
        assert_eq!(cfg.collapse_threshold_tokens, 2000);
    }

    /// When COLLAPSE_MODEL is not set, the default must be "mistralai/mistral-small".
    #[test]
    #[serial]
    fn test_config_collapse_model_default() {
        for var in [
            "HOST",
            "PORT",
            "DATABASE_URL",
            "LOG_LEVEL",
            "OPENROUTER_API_KEY",
            "OPENROUTER_MODEL",
            "OPENROUTER_BASE_URL",
            "OLLAMA_BASE_URL",
            "OLLAMA_MODEL",
            "AUTH_ENABLED",
            "AUTH_ISSUER_URL",
            "AUTH_CLIENT_ID",
            "AUTH_CLIENT_SECRET",
            "AUTH_REDIRECT_URL",
            "JWT_SECRET",
            "OPENWEATHER_API_KEY",
            "BRIEFING_TIME",
            "CONSOLIDATION_TIME",
            "TRAVEL_PREP_DAYS_BEFORE",
            "COLLAPSE_THRESHOLD_TOKENS",
            "COLLAPSE_MODEL",
        ] {
            env::remove_var(var);
        }

        let cfg = Config::from_env();
        assert_eq!(cfg.collapse_model, "mistralai/mistral-small");
    }
}
