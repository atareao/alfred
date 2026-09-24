# Tasks — f2-api-core

## TDD Task Checklist

### Models
- [ ] **2.1** Crear `src/models/mod.rs` con re-exports
- [ ] **2.1** Crear `src/models/conversation.rs`
- [ ] **2.1** Crear `src/models/message.rs` con MessageRole enum
- [ ] **2.1** Crear `src/models/profile.rs`
- [ ] **2.1** Crear `src/models/memory.rs`
- [ ] **2.1** Crear `src/models/tool.rs`
- [ ] **2.1** Crear `src/models/pagination.rs`
- [ ] **2.1** Crear `src/errors.rs` con AppError

### Repos
- [ ] **2.2** Crear `src/db/repos/mod.rs`
- [ ] **2.2** Crear `src/db/repos/conversations.rs`
- [ ] **2.2** Crear `src/db/repos/messages.rs`
- [ ] **2.2** Crear `src/db/repos/profiles.rs`
- [ ] **2.2** Crear `src/db/repos/memories.rs`
- [ ] **2.2** Crear `src/db/repos/tools.rs`

### Routes & Handlers
- [ ] **2.3** Crear `src/routes/mod.rs`
- [ ] **2.3** Crear `src/handlers/mod.rs`
- [ ] **2.3** Crear `src/routes/conversations.rs` + `src/handlers/conversations.rs`
- [ ] **2.3** Crear `src/routes/messages.rs` + `src/handlers/messages.rs`
- [ ] **2.3** Crear `src/routes/profile.rs` + `src/handlers/profile.rs`
- [ ] **2.3** Crear `src/routes/memories.rs` + `src/handlers/memories.rs`
- [ ] **2.3** Crear `src/routes/tools.rs` + `src/handlers/tools.rs`

### Integration
- [ ] **2.4** Actualizar `src/lib.rs` para montar todas las rutas
- [ ] **2.4** Actualizar `src/main.rs` si es necesario

### Tests
- [ ] **2.5** Tests de integración: `tests/api/conversations.rs`
- [ ] **2.5** Tests de integración: `tests/api/messages.rs`
- [ ] **2.5** Tests de integración: `tests/api/profile.rs`
- [ ] **2.5** Tests de integración: `tests/api/memories.rs`
- [ ] **2.5** Tests de integración: `tests/api/tools.rs`

### Verification
- [ ] **2.6** `cargo build` pasa
- [ ] **2.6** `cargo test` pasa (tests legacy + nuevos)
- [ ] **2.6** `cargo clippy -- -D warnings` pasa
- [ ] **2.6** `cargo fmt --check` pasa