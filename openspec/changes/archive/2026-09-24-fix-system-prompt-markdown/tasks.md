# Fix System Prompt Markdown — Task Checklist

## TDD Tasks

### Grupo A: System prompt
- [x] **1.1** RED: tests existentes verifican que el prompt contiene "sarcástico" y "burlón" (ya pasan)
- [x] **1.2** GREEN: actualizar system prompt template en `agent.rs`
- [x] **1.3** REFACTOR: `cargo test`, `cargo clippy -- -D warnings`

### Grupo B: Verificación final
- [x] **2.1** `cargo test` — todos los tests pasan
- [x] **2.2** `cargo clippy -- -D warnings` — cero warnings
- [x] **2.3** `cargo fmt --check` — formato correcto