# Episodic Memory System

## Intent

Reemplazar el sistema actual de memorias (tabla `memories` + `memory_embeddings` JSON) por un sistema de **memoria episódica** de dos capas:

- **Capa A**: Mensajes en bruto (`messages` — ya existe, se modifica)
- **Capa B**: Fichas episódicas sintéticas (`memory` — nueva) generadas por un LLM secundario a partir de bloques de mensajes sin indexar

El sistema actual guarda cada respuesta del asistente como una "memory" de categoría "conversation". El nuevo sistema agrupa mensajes en bloques (~2000 tokens), los resume en una ficha estructurada, y almacena el embedding vectorial usando `sqlite-vec` (`vec0`) en lugar del actual JSON de cosine similarity.

## Scope

### Incluye
1. Nueva tabla `memory` (episodic memory cards) con `tokens_count` exacto y `metadata` JSON
2. Nueva tabla virtual `vec_memory` usando `vec0` (sqlite-vec) para embeddings
3. Modificación de `messages`: FK de `summary_ref` a `memory.id`, índice `idx_messages_unindexed`
4. Nueva migración que crea `memory`, `vec_memory` y altera `messages`
5. Nueva migración que dropea las tablas antiguas `memories`, `memory_embeddings`, `memories_fts`
6. Nuevo worker `EpisodicMemoryWorker` (trigger por canal al insertar mensaje + timer cada 30 min) que genera fichas vía LLM y las persiste
7. Integración del worker en `WorkerPool`
8. Wire del canal en handler `create_message` y orquestador (`Agent`)
9. RAG query: búsqueda por similitud coseno en `vec_memory` + filtro por presupuesto de tokens
10. Prompt de generación de ficha episódica (archivista)
11. **Nuevos endpoints de stats de memoria**: total memorias, tokens acumulados, ratio de mensajes indexados
12. **Actualización del dashboard de stats** (frontend) con tarjetas de memoria
13. **Actualización del README** con la arquitectura de memoria episódica

### No incluye
- Frontend de gestión de memoria (se hará después)
- Session summary (se deja como placeholder)

## Impacto

### Tablas nuevas
- `memory` — reemplaza a `memories`
- `vec_memory` — reemplaza a `memory_embeddings`

### Tablas eliminadas
- `memories` (se migran datos si existen)
- `memory_embeddings`
- `memories_fts`

### Tablas modificadas
- `messages`: se añade FK en `summary_ref` → `memory.id`

### Código afectado
- `src/db/repos/memories.rs` → reemplazar por `src/db/repos/memory.rs` (`MemoryRepo`)
- `src/db/vector.rs` → reemplazar por uso directo de `vec0`
- `src/db/fts.rs` → eliminar `memories_fts`
- `src/services/memory_service.rs` → reemplazar lógica
- `src/services/embedding_service.rs` → reemplazar lógica
- `src/workers/memory_worker.rs` → reemplazar por `EpisodicMemoryWorker`
- `src/handlers/memories.rs` → mantener endpoints adaptados a nuevo `memory`
- `src/models/memory.rs` → nuevo modelo `Memory` (episodic card)
- `src/orchestrator/context_builder.rs` → RAG real contra `vec_memory`
- `Cargo.toml` → activar features de `vec0`

### Tests afectados
- `src/db/repos/memories.rs` tests → reescribir para `memory`
- `tests/api/memories.rs` → actualizar
- `src/workers/memory_worker.rs` tests → reescribir
- `src/db/vector.rs` tests → reemplazar por tests de `vec0`
- `src/db/fts.rs` tests → eliminar tests de `memories_fts`