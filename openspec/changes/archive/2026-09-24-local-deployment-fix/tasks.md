# Local Deployment Fix — Task Checklist

## TDD Tasks

### Grupo A: Backend (main.rs)
- [x] **1.1** RED: test que `main()` usa `Config::from_env()` + `AppState::new_with_orchestrator()`
- [x] **1.2** GREEN: reescribir `main.rs` para usar `Config::from_env()` + `AppState::new_with_orchestrator()`
- [x] **1.3** REFACTOR: `cargo clippy -- -D warnings`, `cargo test`

### Grupo B: Infraestructura (docker-compose.yml + .env)
- [x] **2.1** GREEN: actualizar `docker-compose.yml` con env vars y volumen alfred_data
- [x] **2.2** GREEN: crear `.env` template con valores por defecto
- [x] **2.3** VERIFY: `podman-compose config` valida el YAML

### Grupo C: Verificación final
- [x] **3.1** `cargo test` — todos los tests pasan
- [x] **3.2** `cargo clippy -- -D warnings` — cero warnings
- [x] **3.3** `cargo check` — compilación correcta