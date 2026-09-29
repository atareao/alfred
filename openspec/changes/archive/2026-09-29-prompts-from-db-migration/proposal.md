# Proposal: Prompts del sistema cargados desde la base de datos

## Why

Los tres prompts del sistema están hardcodeados en el código fuente:

- El **system prompt** de Valet (personalidad británica) vive como string literal en `OrchestratorConfig::default()` (`src/orchestrator/agent.rs`).
- El **archivist prompt** vive como constante `ARCHIVIST_PROMPT` (`src/workers/episodic_memory.rs`).
- El **collapse prompt** se siembra desde código en `SettingsRepo::seed_defaults()` (`src/db/repos/settings.rs`).

Esto obliga a recompilar para cambiar la personalidad del asistente y reparte la fuente de verdad entre código y base de datos. Además, en producción `seed_default_settings()` nunca se invoca, por lo que el setting `system_prompt` no existe y el hardcode es siempre el valor efectivo.

## What Changes

- **Nueva migración** `migrations/20260929000001_prompts.sql` que siembra en `settings` las claves `system_prompt`, `archivist_prompt` y `collapse_prompt` con el contenido actualmente hardcodeado.
  - La migración **no sobreescribe** valores existentes no vacíos (respeta personalizaciones del usuario).
  - Rellena valores vacíos o ausentes (bases de datos antiguas con `system_prompt = ''`).
- **BREAKING (interno)**: se elimina el campo `system_prompt_template` de `OrchestratorConfig` y su valor hardcodeado.
- `agent.rs` lee `system_prompt` de la BD en `process_message()` y `process_message_stream()`; si falta o está vacío usa un fallback mínimo genérico y loguea un warning.
- `episodic_memory.rs` elimina `ARCHIVIST_PROMPT` y lee `archivist_prompt` de la BD; sustituye `{{ BLOQUE_DE_MENSAJES }}` por el bloque de mensajes.
- `pool.rs` deja de usar el string hardcodeado de fallback para `collapse_prompt` (la migración garantiza el valor).
- `SettingsRepo::seed_defaults()` deja de sembrar `system_prompt` y `collapse_prompt` (ahora los siembra la migración).
- `routes/settings.rs` elimina `system_prompt_default` (ya no hay template hardcodeado que exponer).
- **Frontend**: la pestaña "Prompt" pasa a ser **"Prompts"** con tres sub-pestañas (System, Archivist, Collapse), una por prompt. Cada sub-pestaña tiene un `TextArea` editable; los tres valores se cargan de `GET /settings` y se guardan con `PUT /settings` en la base de datos.

## Capabilities

### New Capabilities

<!-- Ninguna -->

### Modified Capabilities

- `db/schema`: la migración siembra los prompts del sistema en `settings` y respeta personalizaciones existentes.
- `orchestrator/agent`: el system prompt se carga desde la BD en lugar de un template hardcodeado.
- `workers`: los prompts de collapse y archivist se cargan desde `settings` en lugar de constantes hardcodeadas.
- `frontend`: la pestaña "Prompt" pasa a ser "Prompts" con tres sub-pestañas (System, Archivist, Collapse), cada una editable y persistida en `settings`.

## Impact

- **Migración**: `migrations/20260929000001_prompts.sql` (nueva).
- **Backend Rust**: `src/orchestrator/agent.rs`, `src/workers/episodic_memory.rs`, `src/workers/pool.rs`, `src/db/repos/settings.rs`, `src/routes/settings.rs`.
- **Frontend**: `frontend/src/components/SettingsDialog.tsx`, `frontend/src/hooks/useSettings.ts`, `frontend/src/types/index.ts`.
- **Tests**: `src/orchestrator/agent.rs`, `src/db/repos/settings.rs`, `src/workers/episodic_memory.rs`, `tests/db/migrations.rs`, `frontend/src/test/SettingsDialog.test.tsx`.
- **API**: se elimina la clave `system_prompt_default` de `GET /settings`. `GET /settings` ya devuelve `system_prompt`, `archivist_prompt` y `collapse_prompt`; `PUT /settings` ya persiste claves arbitrarias.
- **Sin cambios** en el esquema de tablas (solo `INSERT`/`UPDATE` de filas en `settings`).