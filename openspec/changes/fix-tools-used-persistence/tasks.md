# TDD Task Checklist: fix-tools-used-persistence

## 1. RED - Escribir tests que fallen
- [ ] 1.1 Añadir `tools_used` a `CreateMessage` struct en Rust
- [ ] 1.2 Añadir `tools_used` a `CreateMessage` interface en TypeScript
- [ ] 1.3 Actualizar handler `create_message` para aceptar y persistir `tools_used`
- [ ] 1.4 Verificar test existente de `test_process_message_stream_adds_tool_footer` sigue pasando

## 2. GREEN - Implementar
- [ ] 2.1 Modificar `src/models/message.rs` — añadir `pub tools_used: Option<String>` a `CreateMessage`
- [ ] 2.2 Modificar `src/handlers/messages.rs` — pasar `body.tools_used.as_deref()` a `MessagesRepo::create`
- [ ] 2.3 Modificar `frontend/src/types/index.ts` — añadir `tools_used?: string` a `CreateMessage`
- [ ] 2.4 Ejecutar `cargo test` y `cargo check`

## 3. REFACTOR - Limpiar
- [ ] 3.1 Ejecutar `cargo clippy -- -D warnings`
- [ ] 3.2 Ejecutar `cargo fmt --check`
- [ ] 3.3 Verificar tests de integracion (si existen)
- [ ] 3.4 Archivar change proposal con `openspec archive`