# Tasks: Episodic Memory System

## TDD Task Checklist

### Fase 1: Schema y migraciones
- [ ] 1.1 Nueva migración: crear tabla `memory` con `id`, `content`, `tokens_count`, `created_at`, `metadata`
- [ ] 1.2 Nueva migración: crear tabla virtual `vec_memory` usando `vec0(embedding float[1536] distance_metric=cosine)`
- [ ] 1.3 Nueva migración: ALTER TABLE messages ADD FOREIGN KEY (summary_ref) REFERENCES memory(id) ON DELETE SET NULL
- [ ] 1.4 Nueva migración: CREATE INDEX idx_messages_unindexed ON messages(created_at) WHERE is_indexed = 0
- [ ] 1.5 Nueva migración: CREATE INDEX idx_messages_summary_ref ON messages(summary_ref) WHERE summary_ref IS NOT NULL
- [ ] 1.6 Nueva migración: DROP TABLE IF EXISTS memories, memory_embeddings, memories_fts
- [ ] 1.7 Actualizar `run_migrations()` en schema para incluir nuevas migraciones

### Fase 2: Modelo MemoryRepo
- [ ] 2.1 Crear `Memory` struct (episodic card) en `src/models/memory.rs`
- [ ] 2.2 Crear `MemoryRepo` con `create`, `find_by_id`, `list`, `delete`, `search_by_vector`
- [ ] 2.3 RED: tests para `MemoryRepo::create` con tokens_count
- [ ] 2.4 GREEN: implementar `MemoryRepo::create`
- [ ] 2.5 RED: tests para `MemoryRepo::search_by_vector` (vec0)
- [ ] 2.6 GREEN: implementar `MemoryRepo::search_by_vector`
- [ ] 2.7 RED: tests para `MemoryRepo::list` con paginación + filtro por presupuesto de tokens
- [ ] 2.8 GREEN: implementar `MemoryRepo::list`
- [ ] 2.9 RED: tests para `MemoryRepo::delete`
- [ ] 2.10 GREEN: implementar `MemoryRepo::delete`

### Fase 3: EpisodicMemoryWorker (canal + timer 30 min)
- [ ] 3.1 RED: test para worker loop con `tokio::select!` (channel + interval + shutdown)
- [ ] 3.2 GREEN: implementar worker loop con select! entre 3 fuentes
- [ ] 3.3 RED: test para selección de mensajes sin indexar (WHERE is_indexed = 0)
- [ ] 3.4 GREEN: implementar query de batch (suma tokens >= 2000 OR inactividad > 30min)
- [ ] 3.5 RED: test para overlap (±2 mensajes antes/después)
- [ ] 3.6 GREEN: implementar lógica de overlap en bloque para LLM
- [ ] 3.7 RED: test para generación de ficha vía LLM
- [ ] 3.8 GREEN: implementar llamada LLM con prompt de archivista
- [ ] 3.9 RED: test para inserción de ficha + embedding vec0 + UPDATE messages
- [ ] 3.10 GREEN: implementar persistencia completa del ciclo

### Fase 4: Wiring del canal (handler + orquestador)
- [ ] 4.1 RED: test de handler create_message envía signal por memory_tx
- [ ] 4.2 GREEN: handler envía signal en `tokio::spawn` post-insert
- [ ] 4.3 RED: test de Agent envía signal al persistir mensajes
- [ ] 4.4 GREEN: orquestador pasa memory_tx y envía señal tras cada persistencia

### Fase 5: RAG en ContextBuilder
- [ ] 5.1 RED: test de ContextBuilder consulta vec_memory por similitud
- [ ] 5.2 GREEN: implementar RAG query que suma tokens_count hasta budget
- [ ] 5.3 RED: test de RAG con presupuesto exacto
- [ ] 5.4 GREEN: implementar filtro por presupuesto

### Fase 6: Limpieza de código legacy
- [ ] 6.1 Eliminar `src/db/vector.rs` (reemplazado por vec0)
- [ ] 6.2 Eliminar funciones de `memories_fts` en `src/db/fts.rs`
- [ ] 6.3 Reemplazar `src/services/memory_service.rs` con nueva lógica
- [ ] 6.4 Reemplazar `src/services/embedding_service.rs` con nueva lógica
- [ ] 6.5 Adaptar handlers de memories API
- [ ] 6.6 Ejecutar `cargo clippy -- -D warnings` y `cargo test`

### Fase 7: Stats de memoria (backend + frontend)
- [ ] 7.1 RED: test para endpoint `GET /api/stats/memory`
- [ ] 7.2 GREEN: implementar `StatsRepo::memory_summary()` con total_memories, total_tokens, messages_indexed, messages_total
- [ ] 7.3 GREEN: añadir ruta `/api/stats/memory` en router
- [ ] 7.4 RED: test para frontend MemoryStatsCard
- [ ] 7.5 GREEN: añadir `MemoryStatsCard` al dashboard (StatsDashboard.tsx)

### Fase 8: Feature flags y configuración
- [ ] 8.1 Añadir `MEMORY_BATCH_TOKENS` (default 2000) a Config
- [ ] 8.2 Añadir `MEMORY_INACTIVITY_MINUTES` (default 30) a Config
- [ ] 8.3 Añadir `MEMORY_OVERLAP` (default 2) a Config
- [ ] 8.4 Añadir `MEMORY_POLL_INTERVAL_MINUTES` (default 30) a Config
- [ ] 8.5 Añadir `MEMORY_MODEL` env var (default `"mistralai/mistral-small"`) a Config — **modelo específico para archivar, distinto del modelo de chat**
- [ ] 8.6 Añadir `RAG_BUDGET_TOKENS` (default 2000) a Config

### Fase 9: Documentación
- [ ] 9.1 Actualizar README.md con arquitectura de memoria episódica
- [ ] 9.2 Actualizar README.md con tabla de endpoints de stats de memoria