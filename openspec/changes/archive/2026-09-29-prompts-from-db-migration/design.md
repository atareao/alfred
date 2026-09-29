# Design: Prompts del sistema desde la base de datos

## Context

Ver `proposal.md` — Why. Estado actual relevante:

- `settings` ya existe (`key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT`) y `collapse_prompt` ya se lee de ahí.
- `SettingsRepo::seed_defaults()` se ejecuta en tests pero **no** en producción (`db::init_db` solo siembra tools). Por eso el hardcode de `agent.rs` es el valor efectivo en producción.
- `OrchestratorConfig` es `Clone` y se construye en `AppState::new_with_orchestrator`; el campo `system_prompt_template` solo se usa como fallback y para exponer `system_prompt_default` en la API.
- `ARCHIVIST_PROMPT` es una constante con el placeholder `{{ BLOQUE_DE_MENSAJES }}`.

## Goals / Non-Goals

**Goals:**
- Una única fuente de verdad para los tres prompts: la tabla `settings`.
- Sembrarlos mediante migración, respetando personalizaciones existentes.
- Eliminar el template hardcodeado de `OrchestratorConfig`.

**Non-Goals:**
- No se añade una tabla `prompts` dedicada ni versionado de prompts.
- No se añade endpoint de "reset de prompts a valores por defecto".
- No se cambia el esquema de tablas (solo filas en `settings`).
- No se toca el prompt de otros workers no listados.

## Decisions

### D1. Almacenar los prompts en `settings` (no en tabla dedicada)
`settings` ya existe, ya es editable desde la UI y ya aloja `collapse_prompt`. Reutilizarlo evita nuevo repo, nueva API y nueva UI.
- Alternativa descartada: tabla `prompts` con historial/versionado → sobre-ingeniería para el requisito actual.

### D2. Migración con upsert condicional
```sql
INSERT INTO settings (key, value, updated_at)
VALUES ('system_prompt', '<prompt>', datetime('now'))
ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')
WHERE settings.value = '' OR settings.value IS NULL;
```
- Rellena claves ausentes y valores vacíos (bases antiguas con `system_prompt = ''`).
- No pisa personalizaciones no vacías.
- Alternativa descartada: `INSERT OR IGNORE` → nunca rellenaría el `''` existente.
- Alternativa descartada: `INSERT OR REPLACE` → destruiría personalizaciones.

### D3. Eliminar `system_prompt_template` de `OrchestratorConfig`
El prompt se lee de `settings.system_prompt` en `process_message()` y `process_message_stream()`. Si falta o está vacío se usa una constante mínima `DEFAULT_SYSTEM_PROMPT_FALLBACK` ("You are Valet, a helpful AI assistant.") y se loguea `warn!`.
- Alternativa descartada: mantener el hardcode como fallback → contradice el objetivo de fuente única.

### D4. `archivist_prompt` se lee en `call_llm()`
`call_llm()` ya recibe `db: &SqlitePool`. Se lee `archivist_prompt`, se sustituye `{{ BLOQUE_DE_MENSAJES }}` y se usa como system message. Fallback mínimo si falta.
- Alternativa descartada: leerlo una vez en `start()` y pasarlo por config → complica la firma y el worker ya tiene acceso a `db`.

### D5. `collapse_prompt`: eliminar el string hardcodeado de `pool.rs`
La migración garantiza el valor. `pool.rs` mantiene la lectura de settings y usa un fallback mínimo si falta.

### D6. `seed_defaults()` deja de sembrar `system_prompt` y `collapse_prompt`
La migración es la fuente. Se mantienen `max_window_tokens`, `message_page_size` y las API keys.

### D7. API: eliminar `system_prompt_default`
El frontend ya recibe `settings.system_prompt` con el valor real. El placeholder separado queda muerto. `resetToDefaults` deja de vaciar `system_prompt` (vaciarlo activaría el fallback mínimo).

### D8. UI: pestaña "Prompts" con tres sub-pestañas
La tab "Prompt" actual se reemplaza por "Prompts", que contiene un `Tabs` anidado con tres items: System, Archivist y Collapse. Cada item monta un `TextArea` de 10 filas.
- Los tres campos viven en el mismo `settingsForm`; el botón "Guardar" envía los tres valores en una sola llamada a `PUT /settings`.
- Se usa `forceRender: true` en los items del `Tabs` anidado para que los tres `Form.Item` estén registrados aunque el usuario no visite todas las sub-pestañas (evita perder valores al guardar).
- Alternativa descartada: tres formularios independientes con guardado por sub-pestaña → más llamadas y más estado; el usuario pidió tres pestañas, no tres guardados.
- Alternativa descartada: un único TextArea con selector → peor UX y no cumple "tres pestañas".

## Risks / Trade-offs

- **Escapado de comillas simples en el SQL de la migración** (el prompt contiene `Do's and Don'ts`) → escapar `'` como `''`; cubrir con test que lea el valor y verifique que contiene "Do's".
- **Bases de datos existentes con `system_prompt = ''`** → el upsert condicional lo rellena; test específico.
- **`seed_defaults` en tests se ejecuta tras migrar** → al quitar `system_prompt`/`collapse_prompt` de ahí, los tests que los esperan deben seguir pasando porque la migración ya los sembró.
- **`sqlx` valida checksums de migraciones aplicadas** → no se puede borrar/editar una migración ya aplicada en producción. Rollback = nueva migración que revierta valores, no borrar el archivo.
- **Lectura de `archivist_prompt` por llamada al LLM** → una query extra por batch; despreciable frente a la llamada LLM.
- **Reset de ajustes ya no restaura el prompt** → documentado; el usuario puede pegar el prompt por defecto manualmente. Un endpoint de reset queda como no-goal.
- **Campos de sub-pestañas no visitadas no registrados en el Form** → `forceRender: true` en los items del `Tabs` anidado garantiza que los tres campos se envíen siempre.

## Migration Plan

1. Añadir `migrations/20260929000001_prompts.sql` (se aplica automáticamente en `run_migrations` al arrancar).
2. Desplegar el código que lee los prompts de `settings`.
3. Rollback: si hay que revertir, añadir una migración nueva que restaure los valores previos; **no** borrar la migración aplicada (rompería el checksum de sqlx).

## Open Questions

Ninguna que bloquee el diseño.