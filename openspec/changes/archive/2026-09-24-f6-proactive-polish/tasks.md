# F6: Tareas Proactivas + Pulido — Task Checklist

## TDD Tasks

### Grupo A: Infraestructura
- [x] **6.7** Logging estructurado con tracing (`src/telemetry.rs`)
  - [x] RED: test que init_tracing no panic
  - [x] GREEN: implementar init_tracing con tracing-subscriber
  - [ ] REFACTOR: integrar en main.rs

- [x] **6.8** Config desde entorno (`src/config.rs`, `.env.example`)
  - [x] RED: test de Config::from_env con defaults
  - [x] GREEN: implementar Config con todas las variables
  - [ ] REFACTOR: reemplazar std::env::var en lib.rs por Config

- [x] **6.9** CORS middleware
  - [x] RED: test de CORS headers en respuesta
  - [x] GREEN: añadir CorsLayer a app_with_state
  - [x] REFACTOR: verificar tests de integración

### Grupo B: Workers
- [x] **6.1** WorkerPool con canal broadcast (`src/workers/`)
  - [x] RED: test de start/shutdown
  - [x] GREEN: implementar WorkerPool con tokio::spawn
  - [x] REFACTOR: clippy, fmt

- [ ] **6.2** Briefing Matutino (`src/workers/briefing.rs`)
  - [ ] RED: test de generación de briefing
  - [ ] GREEN: implementar MorningBriefingWorker
  - [ ] REFACTOR: clippy, fmt

- [x] **6.3** Detección de Conflictos (`src/workers/conflict_detector.rs`)
  - [x] RED: test de detección de solapamientos
  - [x] GREEN: implementar ConflictDetector
  - [x] REFACTOR: clippy, fmt

- [ ] **6.4** Preparación de Viajes (`src/workers/travel_prep.rs`)
  - [ ] RED: test de preparación antes de viaje
  - [ ] GREEN: implementar TravelPrepWorker
  - [ ] REFACTOR: clippy, fmt

- [ ] **6.5** Consolidación Nocturna (`src/workers/memory_worker.rs`)
  - [ ] RED: test `test_consolidate_empty_db` — DB vacía devuelve "Todo en orden"
  - [ ] RED: test `test_consolidate_cleans_orphaned_embeddings` — limpia embeddings huérfanos
  - [ ] GREEN: implementar `MemoryConsolidator` con `new()` y `consolidate()`
  - [ ] REFACTOR: clippy, fmt

### Grupo C: Data & Deploy
- [x] **6.10** Seed data (`src/bin/seed.rs`)
  - [x] RED: test de seed en base vacía
  - [x] GREEN: implementar binario seed
  - [x] REFACTOR: verificar datos creados

- [x] **6.11** Export endpoint (`src/routes/export.rs`)
  - [x] RED: test de GET /api/export
  - [x] GREEN: implementar handler de export
  - [x] REFACTOR: clippy, fmt

- [ ] **6.12** Docker compose producción (`docker-compose.prod.yml`)
  - [ ] GREEN: crear multi-stage Dockerfile.prod
  - [ ] GREEN: crear docker-compose.prod.yml con PocketID
  - [ ] REFACTOR: verificar build

### Grupo D: Frontend & Docs
- [ ] **6.6** ProfileEditor en frontend
  - [ ] Verificar que ProfileEditor.tsx existe y funciona
  - [ ] Añadir campos de perfil (dieta, viajes, productividad)
  - [ ] npx tsc --noEmit

- [ ] **6.13** README.md completo
  - [ ] Escribir README con instalación, configuración, uso

- [ ] **6.14** Auditoría final
  - [ ] cargo test
  - [ ] cargo clippy -- -D warnings
  - [ ] cargo fmt --check
  - [ ] npx tsc --noEmit
  - [ ] npx vitest run