# Tasks

## 1. Migración de prompts (RED → GREEN)

- [x] 1.1 Añadir tests en `tests/db/migrations.rs` que verifiquen que `run_migrations()` siembra `system_prompt` (contiene "asistente personal británico"), `archivist_prompt` (contiene `{{ BLOQUE_DE_MENSAJES }}`) y `collapse_prompt` (contiene "Resume el siguiente texto"). Ejecutar `cargo test --test migrations` y confirmar RED.
- [x] 1.2 Crear `migrations/20260929000001_prompts.sql` con upsert condicional (`ON CONFLICT ... WHERE value = '' OR value IS NULL`) y los tres prompts actuales, escapando `'` como `''`. Ejecutar `cargo test --test migrations` y confirmar GREEN.
- [x] 1.3 Añadir tests de "valor vacío existente se rellena" y "personalización no vacía se respeta"; ejecutar `cargo test --test migrations` y confirmar GREEN.

## 2. System prompt desde la BD (RED → GREEN)

- [x] 2.1 Añadir/ajustar tests en `src/orchestrator/agent.rs` que verifiquen que `process_message()` y `process_message_stream()` usan `settings.system_prompt`, y que hay fallback mínimo no vacío si falta. Ejecutar `cargo test --lib orchestrator::agent` y confirmar RED.
- [x] 2.2 Eliminar el campo `system_prompt_template` de `OrchestratorConfig` y su valor hardcodeado; leer `system_prompt` de `settings` con fallback mínimo `DEFAULT_SYSTEM_PROMPT_FALLBACK` + `warn!`. Ejecutar `cargo test --lib orchestrator::agent` y confirmar GREEN.
- [x] 2.3 Actualizar `test_orchestrator_config_defaults` para que no referencie `system_prompt_template`. Ejecutar `cargo test --lib orchestrator::agent` y confirmar GREEN.

## 3. Archivist prompt desde la BD (RED → GREEN)

- [x] 3.1 Añadir test en `src/workers/episodic_memory.rs` que verifique que `call_llm()` usa `settings.archivist_prompt` y sustituye `{{ BLOQUE_DE_MENSAJES }}`. Ejecutar `cargo test --lib workers::episodic_memory` y confirmar RED.
- [x] 3.2 Eliminar la constante `ARCHIVIST_PROMPT`; leer `archivist_prompt` de `settings` con fallback mínimo + `warn!`. Ejecutar `cargo test --lib workers::episodic_memory` y confirmar GREEN.

## 4. Collapse prompt y seed_defaults

- [x] 4.1 Eliminar `system_prompt` y `collapse_prompt` de `SettingsRepo::seed_defaults`; ajustar los tests de `src/db/repos/settings.rs` para depender de la migración. Ejecutar `cargo test --lib db::repos::settings` y confirmar GREEN.
- [x] 4.2 Simplificar `src/workers/pool.rs`: eliminar el string hardcodeado de fallback de `collapse_prompt` (la migración garantiza el valor). Ejecutar `cargo test --lib workers::pool` y confirmar GREEN.

## 5. API y frontend

- [x] 5.1 Eliminar la clave `system_prompt_default` de `src/routes/settings.rs`; ajustar/eliminar tests que la referencien. Ejecutar `cargo test` y confirmar GREEN.
- [x] 5.2 Añadir tests en `frontend/src/test/SettingsDialog.test.tsx` que verifiquen las tres sub-pestañas (System, Archivist, Collapse), la carga de los tres prompts desde `GET /settings` y el guardado de los tres vía `PUT /settings`. Ejecutar `cd frontend && npx vitest run` y confirmar RED.
- [x] 5.3 Reemplazar la tab "Prompt" por "Prompts" con un `Tabs` anidado de tres items (`forceRender: true`), cada uno con su `TextArea`; actualizar `handleSettingsSubmit` para enviar `system_prompt`, `archivist_prompt` y `collapse_prompt`. Ejecutar `cd frontend && npx vitest run` y confirmar GREEN.
- [x] 5.4 Actualizar `useSettings.ts` (`resetToDefaults` no vacía `system_prompt`) y `types/index.ts` si aplica. Ejecutar `cd frontend && npx tsc --noEmit && npx vitest run` y confirmar GREEN.

## 6. Verificación final

- [x] 6.1 Ejecutar `cargo fmt --check`, `cargo clippy -- -D warnings` y `cargo test`; confirmar todo verde.
- [x] 6.2 Ejecutar `cd frontend && npx tsc --noEmit && npm run build`; confirmar verde.
- [x] 6.3 Ejecutar `openspec validate prompts-from-db-migration --strict`; confirmar válido.