# Spec Delta: db/schema

## ADDED Requirements

### Requirement: Migración siembra los prompts del sistema en settings

**Given** una base de datos recién migrada
**When** se ejecuta `run_migrations()`
**Then** la tabla `settings` SHALL contener las claves `system_prompt`, `archivist_prompt` y `collapse_prompt`
**And** `system_prompt` SHALL contener el prompt de personalidad de Valet (con "asistente personal británico", "Modo Conciso (Predeterminado)", "Expandido" y "Emojis")
**And** `archivist_prompt` SHALL contener el prompt del archivista (con "archivista de memoria" y el placeholder `{{ BLOQUE_DE_MENSAJES }}`)
**And** `collapse_prompt` SHALL contener el prompt de resumen (con "Resume el siguiente texto")
**And** la migración SHALL NOT sobreescribir valores existentes no vacíos (personalizaciones del usuario)
**And** la migración SHALL rellenar valores ausentes o vacíos

#### Scenario: Base de datos nueva recibe los tres prompts
- **WHEN** se ejecuta `run_migrations()` sobre una base de datos vacía
- **THEN** `SELECT value FROM settings WHERE key='system_prompt'` devuelve un valor no vacío que contiene "asistente personal británico"
- **AND** `SELECT value FROM settings WHERE key='archivist_prompt'` devuelve un valor no vacío que contiene "{{ BLOQUE_DE_MENSAJES }}"
- **AND** `SELECT value FROM settings WHERE key='collapse_prompt'` devuelve un valor no vacío que contiene "Resume el siguiente texto"

#### Scenario: Valor vacío existente se rellena
- **GIVEN** una base de datos con `settings.system_prompt = ''`
- **WHEN** se ejecuta la migración de prompts
- **THEN** `settings.system_prompt` pasa a contener el prompt por defecto no vacío

#### Scenario: Personalización existente se respeta
- **GIVEN** una base de datos con `settings.system_prompt = 'Mi prompt personalizado'`
- **WHEN** se ejecuta la migración de prompts
- **THEN** `settings.system_prompt` sigue siendo `'Mi prompt personalizado'`

#### Scenario: Migración idempotente
- **WHEN** se ejecuta `run_migrations()` dos veces seguidas
- **THEN** no se produce error
- **AND** los tres prompts siguen presentes con un único valor por clave