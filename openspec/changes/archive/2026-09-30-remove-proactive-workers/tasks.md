# Tasks

## 1. Eliminar los módulos de worker

- [x] 1.1 Borrar `src/workers/briefing.rs`, `src/workers/conflict_detector.rs`, `src/workers/travel_prep.rs` y `src/workers/memory_worker.rs`.
- [x] 1.2 Actualizar `src/workers/mod.rs` retirando `pub mod briefing;`, `pub mod conflict_detector;`, `pub mod travel_prep;` y `pub mod memory_worker;`.

## 2. Limpiar el WorkerPool

- [x] 2.1 En `src/workers/pool.rs`, retirar los imports de `BriefingWorker`, `ConflictDetector` y `TravelPrepWorker`.
- [x] 2.2 Retirar del struct `WorkerPool` los campos `briefing`, `conflict_detector`, `travel_prep` y `memory_consolidator`.
- [x] 2.3 Retirar los cuatro bloques `tokio::spawn` correspondientes en `WorkerPool::start()` y sus entradas en el `Self { ... }`.
- [x] 2.4 Retirar el aborto de esos handles en `WorkerPool::shutdown()`.
- [x] 2.5 Retirar los tests que referencian esos workers (`test_worker_pool_start`, `test_worker_pool_shutdown` y los tres tests `[RED]` del final) y las aserciones correspondientes.
- [x] 2.6 Retirar de `test_config()` los campos `briefing_time`, `consolidation_time` y `travel_prep_days_before`.

## 3. Limpiar Config

- [x] 3.1 Retirar de `src/config.rs` los campos `briefing_time`, `consolidation_time` y `travel_prep_days_before`, su parseo de entorno (`BRIEFING_TIME`, `CONSOLIDATION_TIME`, `TRAVEL_PREP_DAYS_BEFORE`) y las aserciones de test que los usan.

## 4. Verificación

- [x] 4.1 `cargo fmt --check` limpio.
- [x] 4.2 `cargo clippy --all-targets -- -D warnings` limpio (sin código muerto).
- [x] 4.3 `cargo test` verde.
- [x] 4.4 `openspec validate remove-proactive-workers --strict` válido.
