# Fix LLM Tool Calls — Task Checklist

## TDD Tasks

### Grupo A: OpenRouterProvider — parsear tool_calls
- [x] **1.1** RED: test que OpenRouter parsea tool_calls de la respuesta
- [x] **1.2** GREEN: implementar parsing de tool_calls en `OpenRouterProvider::chat()`
- [x] **1.3** REFACTOR: `cargo clippy -- -D warnings`, `cargo test`

### Grupo B: OllamaProvider — parsear tool_calls
- [x] **2.1** RED: test que Ollama parsea tool_calls de la respuesta
- [x] **2.2** GREEN: implementar parsing de tool_calls en `OllamaProvider::chat()`
- [x] **2.3** REFACTOR: `cargo clippy -- -D warnings`, `cargo test`

### Grupo C: Verificación final
- [x] **3.1** `cargo test` — todos los tests pasan (399 tests)
- [x] **3.2** `cargo clippy -- -D warnings` — cero warnings
- [x] **3.3** `cargo fmt --check` — formato correcto