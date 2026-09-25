# TDD Task Checklist: tool-max-retries

## RED phase — Write failing tests first

- [x] **RED 1**: Test con mock tool que falla 4 veces seguidas → 3 ejecuciones reales, la 4ª es bloqueada, el LLM recibe mensaje de límite alcanzado

  - **Resultado RED**: `call_count` = 4 (sin límite), test falla

## GREEN phase — Implement

- [x] **GREEN 1**: Añadir `tool_call_counts: HashMap<String, usize>` en `process_message_stream()`
- [x] **GREEN 2**: Antes de `registry.execute()`, verificar contador y bloquear si >= 3
- [x] **GREEN 3**: Aplicar el mismo cambio en `process_message()` (no-streaming)

## REFACTOR phase

- [x] **REFACTOR**: `cargo clippy -- -D warnings` — zero warnings
- [x] **REFACTOR**: `cargo test` — all tests green (~415 tests)
- [x] **REFACTOR**: `cargo fmt --check` — sin diferencias