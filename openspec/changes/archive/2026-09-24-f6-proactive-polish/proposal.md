# F6: Tareas Proactivas + Pulido + DX

## Intent

Completar Alfred con las funcionalidades que lo hacen un sistema autónomo y preparado para producción:

- **Workers proactivos**: briefing matutino, detección de conflictos, preparación de viajes, consolidación nocturna
- **Infraestructura**: logging estructurado, configuración desde entorno, CORS
- **Data**: seed data para desarrollo, export de datos, Docker producción
- **Frontend**: ProfileEditor funcional
- **Documentación**: README.md completo
- **Auditoría final**: clippy, test, tsc, vitest

## Scope

| # | Tarea | Archivos | Grupo |
|---|-------|----------|-------|
| 6.1 | WorkerPool con canal mpsc | `src/workers/mod.rs`, `src/workers/pool.rs` | B |
| 6.2 | Briefing Matutino (cron diario) | `src/workers/briefing.rs` | B |
| 6.3 | Detección de Conflictos de Agenda | `src/workers/conflict_detector.rs` | B |
| 6.4 | Preparación de Viajes | `src/workers/travel_prep.rs` | B |
| 6.5 | Consolidación Nocturna | `src/workers/memory_worker.rs` | B |
| 6.6 | ProfileEditor en frontend | `frontend/src/components/ProfileEditor.tsx` | D |
| 6.7 | Logging estructurado con tracing | `src/telemetry.rs` | A |
| 6.8 | Config desde entorno (config.rs + .env) | `src/config.rs`, `.env.example` | A |
| 6.9 | CORS middleware | `src/lib.rs` | A |
| 6.10 | Seed data para desarrollo | `src/bin/seed.rs` | C |
| 6.11 | Export de datos: GET /api/export | `src/routes/export.rs`, `src/handlers/export.rs` | C |
| 6.12 | Docker compose producción (multi-stage + PocketID) | `docker-compose.prod.yml` | C |
| 6.13 | README.md completo | `README.md` | D |
| 6.14 | Auditoría final: cargo clippy, cargo test, npx tsc, npx vitest | — | D |

## Impact

- **Nuevas dependencias**: ninguna (tracing, tokio, reqwest ya están)
- **Nuevos módulos**: `src/workers/`, `src/config.rs`, `src/telemetry.rs`, `src/middleware/`, `src/bin/seed.rs`
- **Nuevos endpoints**: `GET /api/export`
- **Nuevos workers**: 4 workers asíncronos con tokio::spawn + interval
- **Sin cambios en schema**: los workers leen datos existentes, no crean tablas nuevas