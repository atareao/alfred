# Proposal: Eliminar los workers proactivos no funcionales

## Why

Cuatro de los seis workers del `WorkerPool` son andamiaje que no aporta valor:

- **BriefingWorker** consulta un `profile_id` hardcodeado (`"profile-id"`) que no existe en la base de datos (los perfiles reales son UUIDs), por lo que el briefing siempre sale vacío. Además su `weather_api_key` nunca se usa.
- **ConflictDetector** tiene lógica real, pero solo escribe los avisos en `tracing`; no entrega nada al usuario.
- **TravelPrepWorker** devuelve markdown de relleno: `prepare_for_trip()` no llama al clima ni a `search_places`.
- **Memory consolidator** es un no-op puro (`tracing::info!("...tick")` + `let _ = &db;`).

Ninguno respeta su hora configurada (`briefing_time`, `consolidation_time`, `travel_prep_days_before`): el pool usa intervalos fijos (60s/120s/300s/600s) y esas claves de `Config` no se leen en ningún sitio. El resultado es CPU consumida cada minuto para escribir en los logs.

Se eliminan por completo. La capa proactiva (scheduler + entrega) se rediseñará más adelante si se retoma.

## What Changes

- **Eliminar** los módulos `src/workers/briefing.rs`, `src/workers/conflict_detector.rs`, `src/workers/travel_prep.rs` y `src/workers/memory_worker.rs` (stub deprecado).
- **Eliminar** del `WorkerPool` los campos `briefing`, `conflict_detector`, `travel_prep` y `memory_consolidator`, sus bloques `tokio::spawn`, su manejo en `shutdown()` y los tests que los referencian.
- **Eliminar** de `Config` los campos `briefing_time`, `consolidation_time` y `travel_prep_days_before` (y su parseo de entorno y tests), al quedar sin uso.
- **Actualizar** `src/workers/mod.rs` para retirar las declaraciones `pub mod`.
- **Actualizar** las specs de OpenSpec: retirar los requirements de briefing, conflictos y viajes, y el escenario que los enumera.

## Capabilities

### New Capabilities

<!-- Ninguna -->

### Modified Capabilities

- `workers`: se retiran los requirements de Briefing, Conflict-detector y Travel-prep, y se actualiza el escenario de arranque del pool para listar solo Collapse y EpisodicMemory.
- `workers/travel_prep`: se retira la capability completa.

## Impact

- **Borrados**: `src/workers/briefing.rs`, `src/workers/conflict_detector.rs`, `src/workers/travel_prep.rs`, `src/workers/memory_worker.rs`.
- **Editados**: `src/workers/mod.rs`, `src/workers/pool.rs`, `src/config.rs`.
- **Specs**: `openspec/specs/workers/spec.md`, `openspec/specs/workers/travel_prep/spec.md`.
- **Sin cambios** en API, base de datos, frontend ni dependencias.
- **Fuera de alcance**: `src/services/memory_service.rs` y `src/services/embedding_service.rs` (también deprecados, pero no son workers).
