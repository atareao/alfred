# Backend Spec Delta — f1-scaffolding

### ADDED: Cargo.toml dependencies

```toml
[package]
name = "alfred"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = "0.7"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.31", features = ["bundled"] }
sqlite-vec = "0.1"
tower-http = { version = "0.5", features = ["cors"] }
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
anyhow = "1"
thiserror = "1"
dotenvy = "0.15"
```

### ADDED: Health check endpoint

`GET /api/health` response:
```json
{
  "status": "ok",
  "version": "0.1.0",
  "db": "connected"
}
```

### ADDED: AppState

```rust
pub struct AppState {
    pub db: Arc<Mutex<rusqlite::Connection>>,
}
```

### ADDED: SQLite schema

5 core tables (conversations, messages, profiles, memories, tools), 2 vec0 virtual tables (message_embeddings, memory_embeddings), 2 FTS5 virtual tables (messages_fts, memories_fts).

### ADDED: Scenario: Health check returns 200

- **Given** the Axum server is running
- **When** `GET /api/health` is called
- **Then** returns status 200 with JSON `{"status":"ok","version":"0.1.0","db":"connected"}`

### ADDED: Scenario: Migration creates all tables

- **Given** the database is empty
- **When** run_migrations() is executed
- **Then** tables conversations, messages, profiles, memories, tools exist

### ADDED: Scenario: Migration is idempotent

- **Given** migrations have already run once
- **When** run_migrations() is executed again
- **Then** no error is thrown and tables still exist