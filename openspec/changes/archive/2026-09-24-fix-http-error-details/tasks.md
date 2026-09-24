# Fix HTTP Error Details — Task Checklist

## TDD Tasks

### Grupo A: OpenRouterProvider — leer body en errores HTTP
- [ ] **1.1** RED: tests existentes (verifican error handling genérico, siguen pasando)
- [ ] **1.2** GREEN: leer response body en errores HTTP de OpenRouterProvider::chat()
- [ ] **1.3** REFACTOR: `cargo clippy -- -D warnings`, `cargo test`

### Grupo B: OllamaProvider — leer body en errores HTTP
- [ ] **2.1** RED: tests existentes (verifican error handling genérico, siguen pasando)
- [ ] **2.2** GREEN: leer response body en errores HTTP de OllamaProvider::chat()
- [ ] **2.3** REFACTOR: `cargo clippy -- -D warnings`, `cargo test`

### Grupo C: Verificación final
- [ ] **3.1** `cargo test` — todos los tests pasan
- [ ] **3.2** `cargo clippy -- -D warnings` — cero warnings
- [ ] **3.3** `cargo fmt --check` — formato correcto