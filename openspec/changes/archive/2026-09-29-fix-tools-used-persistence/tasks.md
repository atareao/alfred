# TDD Task Checklist: fix-tools-used-persistence

> **Nota:** este change quedó *stuck* tras implementarse el código en la release 0.7.0.
> Se completó la verificación y se archiva retroactivamente.

## 1. RED - Escribir tests que fallen
- [x] 1.1 Añadir `tools_used` a `CreateMessage` struct en Rust
- [x] 1.2 Añadir `tools_used` a `CreateMessage` interface en TypeScript
- [x] 1.3 Actualizar handler `create_message` para aceptar y persistir `tools_used`
- [x] 1.4 Verificar test existente de `test_process_message_stream_adds_tool_footer` sigue pasando

## 2. GREEN - Implementar
- [x] 2.1 Modificar `src/models/message.rs` — añadir `pub tools_used: Option<String>` a `CreateMessage`
- [x] 2.2 Modificar `src/handlers/messages.rs` — pasar `body.tools_used.as_deref()` a `MessagesRepo::create`
- [x] 2.3 Modificar `frontend/src/types/index.ts` — añadir `tools_used?: string` a `CreateMessage`
- [x] 2.4 Ejecutar `cargo test` y `cargo check`

## 3. REFACTOR - Limpiar
- [x] 3.1 Ejecutar `cargo clippy -- -D warnings`
- [x] 3.2 Ejecutar `cargo fmt --check`
- [x] 3.3 Verificar tests de integración (si existen)
- [x] 3.4 Archivar change proposal con `openspec archive`
