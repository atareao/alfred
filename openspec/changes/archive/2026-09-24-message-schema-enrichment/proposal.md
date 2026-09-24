# Change Proposal: Message Schema Enrichment

## Intent
Enriquecer el modelo `Message` con campos de contaje de tokens, contenido colapsado (resumido), y un flag de indexado. Cuando un mensaje supera un umbral configurable de tokens, se dispara un job en background para colapsar su contenido usando un LLM económico.

## Scope
- **Modelos**: `src/models/message.rs` — nuevos campos en `Message` y `CreateMessage`
- **Base de datos**: `src/db/schema.rs` — migración `ALTER TABLE messages ADD COLUMN`
- **Repo**: `src/db/repos/messages.rs` — actualizar queries de INSERT/SELECT
- **Background job**: Nuevo módulo de workers para el colapso asíncrono
- **Config**: `src/config.rs` — nuevo campo `collapse_threshold_tokens` (default 2000)
- **Tests**: Actualizar tests existentes + nuevos tests de integración

## Impacto
- **Backwards compatibility**: Sí — nuevos campos tienen defaults, columnas se añaden con `ADD COLUMN IF NOT EXISTS`
- **Base de datos**: Migración suave, no rompe datos existentes
- **API**: `Message` en respuestas JSON incluirá los nuevos campos; `CreateMessage` no cambia (el servidor computa tokens automáticamente)

## No incluye (futuro)
- La conexión real a un LLM para el collapse — se deja preparada la infraestructura de workers con un stub
- Búsqueda semántica sobre `collapsed_content`