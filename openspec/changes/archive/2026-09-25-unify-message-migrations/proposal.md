# Unificar migraciones de messages

## Intent

Fusionar `20260925000002_message_enrichment.sql` (ALTER TABLE) dentro del CREATE TABLE
de `messages` en `20260925000001_core_tables.sql`, y eliminar el archivo sobrante.

## Justificación

Actualmente la tabla `messages` se crea en dos pasos:

1. `core_tables.sql` crea la tabla con columnas básicas
2. `message_enrichment.sql` añade 5 columnas vía ALTER TABLE

No hay razón técnica para mantenerlas separadas. Unificarlas simplifica el esquema,
reduce el número de migraciones y hace el schema más legible.

## Scope

- `migrations/20260925000001_core_tables.sql` — modificar CREATE TABLE messages
- `migrations/20260925000002_message_enrichment.sql` — eliminar
- `src/db/schema.rs` — tests existentes ya cubren las columnas, no requieren cambios
- `tests/db/migrations.rs` — tests existentes ya cubren las columnas, no requieren cambios

## Impacto

- **Bases de datos existentes**: No hay (data/ vacío). En producción futura no habrá
  impacto porque solo se toca la migración inicial.
- **sqlx**: Al eliminar #02, fresh databases ejecutarán solo #01 con todas las columnas.
  No hay `_sqlx_migrations` previas, así que no hay riesgo de inconsistencia.
- **Tests**: Los tests existentes de columnas (`test_messages_table_has_new_columns`)
  siguen siendo válidos — verifican columnas en la tabla, no importa si vienen de CREATE
  o ALTER.