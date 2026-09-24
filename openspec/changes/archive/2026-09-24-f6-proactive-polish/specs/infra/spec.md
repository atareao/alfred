# Infraestructura: Logging, Config, CORS

## Contracts

```rust
// ── Config ───────────────────────────────────────────────────────────────

pub struct Config {
    // Server
    pub host: String,           // default: "0.0.0.0"
    pub port: u16,              // default: 3000
    pub database_url: String,   // default: "alfred.db"
    pub log_level: String,      // default: "info"

    // LLM
    pub openrouter_api_key: Option<String>,
    pub openrouter_model: String,       // default: "anthropic/claude-sonnet-20241022"
    pub openrouter_base_url: String,    // default: "https://openrouter.ai/api/v1"
    pub ollama_base_url: String,        // default: "http://localhost:11434"
    pub ollama_model: String,           // default: "llama3.2:3b"

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
    pub briefing_time: String,          // default: "08:15"
    pub consolidation_time: String,     // default: "23:00"
    pub travel_prep_days_before: u32,   // default: 3
}

impl Config {
    pub fn from_env() -> Self;
}

// ── Telemetry ────────────────────────────────────────────────────────────

pub fn init_tracing(log_level: &str);
// Inicializa tracing-subscriber con formato JSON en producción,
// formato humano en desarrollo, y filtro por nivel.

// ── CORS ─────────────────────────────────────────────────────────────────

// Configuración CORS en app_with_state() usando tower-http.
// Permite orígenes desde AUTH_REDIRECT_URL o http://localhost:5173 en dev.
```

## Scenarios

### Config: loads from environment
**Given** environment variables are set  
**When** `Config::from_env()` is called  
**Then** it returns a Config with those values

### Config: uses defaults when env not set
**Given** no environment variables  
**When** `Config::from_env()` is called  
**Then** it returns a Config with default values

### Tracing: initializes subscriber
**Given** a log level  
**When** `init_tracing()` is called  
**Then** the global tracing subscriber is set

### CORS: allows frontend origin
**Given** a request from the frontend origin  
**When** it reaches the API  
**Then** CORS headers allow the request