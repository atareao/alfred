# Fix Tool Call ID — Task Checklist

## TDD Tasks

### Grupo A: Añadir tool_call_id a ChatMessage y serializarlo
- [ ] **1.1** RED: tests que verifican que los mensajes tool incluyen tool_call_id
- [ ] **1.2** GREEN: añadir campo a ChatMessage, serializar en openrouter.rs, pasar en agent.rs
- [ ] **1.3** REFACTOR: `cargo clippy -- -D warnings`, `cargo test`

### Grupo B: Verificación final
- [ ] **2.1** `cargo test` — todos los tests pasan
- [ ] **2.2** `cargo clippy -- -D warnings` — cero warnings
- [ ] **2.3** `cargo fmt --check` — formato correcto