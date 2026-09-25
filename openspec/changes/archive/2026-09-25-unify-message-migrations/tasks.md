# Tasks: unify-message-migrations

## TDD Checklist

### [x] 1. RED — Verificar que los tests actuales pasan antes del cambio
- [x] Ejecutar `cargo test` para confirmar estado GREEN actual

### [x] 2. Merge — Fusionar columnas en CREATE TABLE
- [x] Modificar `migrations/20260925000001_core_tables.sql` añadiendo las columnas
      de enrichment al CREATE TABLE messages
- [x] Eliminar `migrations/20260925000002_message_enrichment.sql`

### [x] 3. GREEN — Verificar que tests siguen pasando
- [x] Ejecutar `cargo test`
- [x] Ejecutar `cargo clippy -- -D warnings`
- [x] Ejecutar `cargo fmt --check`

### [x] 4. Consolidar
- [x] `openspec archive unify-message-migrations`