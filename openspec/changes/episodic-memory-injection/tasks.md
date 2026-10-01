# Tasks

> **Invariante**: ninguna aserción de test existente puede cambiar salvo estas tres excepciones, declaradas a propósito y de forma explícita:
> 1. Las que fijan el formato `[{tags}] {content}`.
> 2. Las que asumen que `vec_memory` guarda JSON.
> 3. Los tests de `!doc` (`test_doc_override`, `test_classify_with_doc`), que desaparecen con la funcionalidad.
>
> Cualquier otro test que se rompa es una regresión, no una actualización.

## 1. Caracterización previa de lo existente (baseline GREEN)

- [ ] 1.1 Ejecutar `cargo test --lib orchestrator::context_builder && cargo test --lib orchestrator::context_classifier && cargo test --lib db::repos::memory` y registrar el resultado como baseline. Confirmar GREEN antes de tocar nada.
- [ ] 1.2 Añadir tests de caracterización que documenten el comportamiento actual: `build()` devuelve `rag_memories` vacío en `SlidingWindow` y `Historical`, y `[Memory context]` aparece solo en `RAG`. Ejecutar `cargo test --lib orchestrator::context_builder && cargo test --lib orchestrator::agent` y confirmar GREEN (caracterización, no comportamiento nuevo).
- [ ] 1.3 Documentar en el test de caracterización las tres únicas excepciones a la invariante: `test_rag_with_pool_and_provider_returns_formatted_memories` (formato `[{tags}] {content}`), los tests que asumen `embedding` como JSON en `vec_memory`, y `test_doc_override`/`test_classify_with_doc`. Ejecutar `cargo test --lib orchestrator::context_builder && cargo test --lib db::repos::memory` y confirmar GREEN.

## 2. Dependencia `vec0`, registro de la extensión y fail-fast (RED → GREEN)

- [ ] 2.1 Activar el feature `vec0` de `Cargo.toml` (hoy declarado y sin usar: `sqlite-vec` opcional + feature `vec0`) y añadir `libsqlite3-sys = "0.37"` como dependencia directa, para registrar la extensión sobre la misma instancia que usa `sqlx-sqlite`. Ejecutar `cargo tree -i libsqlite3-sys` y confirmar una **única** instancia `0.37.0` compartida por `sqlx-sqlite` (spike: `sqlite-vec 0.1.9` no depende de `libsqlite3-sys`, así que Cargo unifica).
- [ ] 2.2 Registrar `sqlite3_auto_extension(sqlite3_vec_init)` al arrancar (una sola vez, antes de abrir el pool). Añadir un test que ejecute `SELECT vec_version()` **desde una conexión del pool** y confirme que devuelve `v0.1.9`. Ejecutar `cargo test --lib db` y confirmar RED → GREEN.
- [ ] 2.3 Añadir un test que ejecute `SELECT vec_version()` **desde una conexión nueva** (fuera del pool) y confirme `v0.1.9`, verificando que el registro alcanza a cada conexión al abrirse. Ejecutar `cargo test --lib db` y confirmar GREEN.
- [ ] 2.4 Fail-fast (D11): probar `SELECT vec_version()` al arrancar y, si falla, **no arrancar**, con un error explícito que diga qué falta. Test de que el arranque falla si la extensión no está registrada. Ejecutar `cargo test --lib lib` y confirmar GREEN.

## 3. Migración a la tabla virtual `vec0` y comprobación de dimensión al arrancar (RED → GREEN)

- [x] 3.1 Migración: `CREATE VIRTUAL TABLE vec_memory USING vec0(id TEXT PRIMARY KEY, embedding float[1024] distance_metric=cosine)`, **con la columna `id` textual declarada explícitamente** (sin ella no se puede hacer el JOIN por id con `memory`; el error es `table vec_items has no column named id`). Añadir test que confirme en `sqlite_master` que `vec_memory` es virtual y expone las columnas `id` y `embedding`. Ejecutar `cargo test --lib db::schema` y confirmar RED → GREEN.
- [x] 3.2 Comprobación de alineación de dimensión al arrancar (D10): comparar la dimensión declarada por `vec_memory` (`float[N]`) con `EMBEDDING_DIMENSION`; si difieren, fallar con un mensaje explícito que indique reconstruir el índice. Tests: alineado continúa; desalineado falla. Ejecutar `cargo test --lib db && cargo test --lib config` y confirmar GREEN.
- [x] 3.3 Test de que `vec0` rechaza almacenar un vector de dimensión distinta de la declarada (la deriva de dimensión desaparece estructuralmente). Ejecutar `cargo test --lib db::schema` y confirmar GREEN.
- [x] 3.4 Verificar el JOIN `SELECT m.id, m.content, m.tokens_count, v.distance FROM memory m JOIN vec_memory v ON m.id = v.id WHERE v.embedding MATCH ? AND k = ? ORDER BY v.distance` y que devuelve los campos de `memory` junto con la distancia. Ejecutar `cargo test --lib db::repos::memory` y confirmar GREEN.

## 4. Escritura del vector en `vec0` (worker, repo y reindex) (RED → GREEN)

- [x] 4.1 Actualizar la escritura del embedding al formato binario de `vec0` en `EpisodicMemoryWorker::persist` (`src/workers/episodic_memory.rs`), en `MemoryRepo::create_in_tx` (`src/db/repos/memory.rs`) y en `reindex_all` (`src/embeddings/reindex.rs`). Actualizar de forma explícita los tests que asumían `embedding` como texto JSON (excepción 2 de la invariante). Ejecutar `cargo test --lib workers::episodic_memory && cargo test --lib db::repos::memory && cargo test --lib embeddings::reindex` y confirmar GREEN.
- [x] 4.2 Verificar que `reindex_all` recorre las filas de `memory`, regenera el embedding y reemplaza la fila por `id` en `vec_memory`, devolviendo `ReindexReport { total, updated, failed }`. Ejecutar `cargo test --lib embeddings::reindex` y confirmar GREEN.
- [x] 4.3 Verificar que `memory` sigue siendo la fuente de verdad y que `vec_memory` solo contiene el índice vectorial (JOIN alineado por id). Ejecutar `cargo test --lib db::repos::memory && cargo test --lib workers::episodic_memory` y confirmar GREEN.

## 5. Recuperación KNN, umbral, decaimiento y presupuesto (RED → GREEN)

- [ ] 5.1 Reescribir `MemoryRepo::search_by_vector` para usar `MATCH … AND k = MEMORY_KNN_CANDIDATES ORDER BY distance` con JOIN a `memory`, en lugar de cargar todas las filas y calcular el coseno en Rust. Ejecutar `cargo test --lib db::repos::memory` y confirmar RED.
- [ ] 5.2 Implementar en Rust el pipeline documentado en `design.md`: `similitud = 1 - distance`; descartar `similitud < SIMILARITY_THRESHOLD` (**aplicado a la similitud, nunca al resultado final** — D3); `final = similitud × exp(-ln2 × días / MEMORY_HALF_LIFE_DAYS)` con `f64::exp()` (**en Rust, no en SQL**: la SQLite de `sqlx` no tiene `exp`); ordenar por `final` descendente; acumular `tokens_count` hasta `RAG_BUDGET_TOKENS`. Tests: distancia 0.0 (similitud 1.0) entra con umbral 0.5; distancia 0.8 (similitud 0.2) se descarta; umbral alto deja el resultado vacío. Ejecutar `cargo test --lib db::repos::memory` y confirmar RED → GREEN.
- [ ] 5.3 Test del decaimiento (D3/D4): una ficha antigua cuya similitud supera el umbral **sigue apareciendo** aunque su `final` sea menor que el de una más reciente; el decaimiento cambia el orden, no la pertenencia. Ejecutar `cargo test --lib db::repos::memory && cargo test --lib orchestrator::context_builder` y confirmar GREEN.
- [ ] 5.4 Corregir el corte por presupuesto de `continue` a `break` (paso 7 del pipeline): con presupuesto 800 y fichas 347/307/248/245/210/164/138, el resultado debe ser 2 fichas (347+307), no 3 (347+307+138). Ejecutar `cargo test --lib db::repos::memory` y confirmar GREEN.
- [ ] 5.5 Test de que `MEMORY_KNN_CANDIDATES` acota las candidatas sin vaciar el resultado cuando existe al menos una ficha por encima del umbral (D5): como la KNN ordena por distancia ascendente y el corte del paso 4 conserva un prefijo, la primera candidata que supera el umbral garantiza resultado. Ejecutar `cargo test --lib db::repos::memory` y confirmar GREEN.

## 6. Los cuatro mandos de memoria en `settings` (RED → GREEN)

- [ ] 6.1 Sembrar en la migración las claves `MEMORY_HALF_LIFE_DAYS = 90`, `SIMILARITY_THRESHOLD = 0.5` (provisional), `RAG_BUDGET_TOKENS = 800` y `MEMORY_KNN_CANDIDATES = 20`, respetando personalizaciones existentes (mismo patrón que los prompts). Tests: base nueva recibe los cuatro; valor existente no vacío se respeta. Ejecutar `cargo test --lib db::schema` y confirmar RED → GREEN.
- [ ] 6.2 Leer los cuatro mandos desde `settings` **en cada consulta** (no al arrancar), de modo que cambiarlos surta efecto sin reiniciar. Tests: cambiar `SIMILARITY_THRESHOLD` entre dos consultas cambia el resultado. Ejecutar `cargo test --lib orchestrator::context_builder && cargo test --lib db::repos::memory` y confirmar RED → GREEN.
- [ ] 6.3 Actualizar el default de `RAG_BUDGET_TOKENS` a `800` en `src/config.rs` y `src/lib.rs`, y documentar los cuatro mandos en `.env.example`. Ejecutar `grep -E 'RAG_BUDGET_TOKENS|MEMORY_HALF_LIFE_DAYS|SIMILARITY_THRESHOLD|MEMORY_KNN_CANDIDATES' .env.example` y confirmar que los cuatro aparecen.
- [ ] 6.4 Añadir tests de presupuesto en `context_builder.rs`: con `rag_budget_tokens = 300` y fichas de 237 tokens, entra la primera y no la segunda. Ejecutar `cargo test --lib orchestrator::context_builder` y confirmar GREEN.

## 7. Panel de ajustes de memoria en el frontend (RED → GREEN)

- [ ] 7.1 Añadir tests en `frontend/src/test/SettingsDialog.test.tsx` que verifiquen el panel "Memoria" con los cuatro campos (`MEMORY_HALF_LIFE_DAYS`, `SIMILARITY_THRESHOLD`, `RAG_BUDGET_TOKENS`, `MEMORY_KNN_CANDIDATES`), su carga desde `GET /settings` y su guardado vía `PUT /settings`. Ejecutar `cd frontend && npx vitest run` y confirmar RED.
- [ ] 7.2 Implementar el panel "Memoria" en `SettingsDialog.tsx` (campos numéricos), actualizar `useSettings.ts` y `types/index.ts` si aplica. Ejecutar `cd frontend && npx tsc --noEmit && npx vitest run` y confirmar GREEN.

## 8. Composición, aislamiento e inyección automática del bloque (RED → GREEN)

- [ ] 8.1 Reescribir `format_memory()` para producir la ficha con ancla temporal (`created_at`) y sin `[{tags}]`; añadir test de que una ficha sin `tags` no contiene `[]` ni `[tags]`. Ejecutar `cargo test --lib orchestrator::context_builder` y confirmar RED.
- [ ] 8.2 Actualizar `test_rag_with_pool_and_provider_returns_formatted_memories` al nuevo formato (excepción 1: única aserción de formato que cambia a propósito). Ejecutar `cargo test --lib orchestrator::context_builder` y confirmar GREEN.
- [ ] 8.3 Añadir en `src/orchestrator/agent.rs` la composición del bloque `<episodic_memory>` dentro de la sección `# CONTEXTO DE MEMORIA EPISÓDICA (CAPA B)`, con la instrucción de que son antecedentes y no parte del turno actual. Añadir test de que el prompt contiene las etiquetas y la instrucción. Ejecutar `cargo test --lib orchestrator::agent` y confirmar RED.
- [ ] 8.4 Implementar la composición y sustituir el literal `format!("[Memory context] {}", memory)` en `process_message` y `process_message_stream`; el bloque se compone **en código**, nunca desde un placeholder de `settings.system_prompt` (D2). Añadir test de que `[Memory context]` ya no aparece. Ejecutar `cargo test --lib orchestrator::agent` y confirmar GREEN.
- [ ] 8.5 Añadir test de que el bloque `<episodic_memory>` va **antes** del primer mensaje del historial. Ejecutar `cargo test --lib orchestrator::agent` y confirmar GREEN.
- [ ] 8.6 Añadir tests en `context_builder.rs` de que `SlidingWindow` y `Historical` recuperan la misma memoria que antes recuperaba `RAG`, cuando existen fichas sobre el umbral. Ejecutar `cargo test --lib orchestrator::context_builder` y confirmar RED → GREEN.
- [ ] 8.7 Añadir test de que un mensaje clasificado como `Override::None` / `SlidingWindow` recibe memoria si supera el umbral. Ejecutar `cargo test --lib orchestrator::context_classifier && cargo test --lib orchestrator::context_builder` y confirmar GREEN.

## 9. Eliminación de la estrategia RAG y de `!doc` (RED → GREEN)

- [ ] 9.1 Eliminar la variante `ContextStrategy::RAG`, su rama en `ContextBuilder::build()`, `Override::Doc` del clasificador y la detección de `!doc` en `classify()`. Ejecutar `cargo check --all-targets` y confirmar que no quedan referencias.
- [ ] 9.2 Eliminar los tests `test_doc_override` y `test_classify_with_doc` (excepción 3: desaparecen con la funcionalidad). Ejecutar `cargo test --lib orchestrator::context_classifier` y confirmar GREEN.
- [ ] 9.3 Añadir tests de que `!historico` y `!reset` siguen gobernando su propia estrategia, ya independiente de la memoria. Ejecutar `cargo test --lib orchestrator::context_classifier` y confirmar GREEN.
- [ ] 9.4 Verificar que el requisito `search_by_vector SHALL warn on embedding dimension mismatch` se retira (delta `REMOVED` en `db/repos`) y que no quedan tests que lo asuman salvo los declarados. Ejecutar `cargo test --lib db::repos::memory` y confirmar GREEN.

## 10. Limpieza de código muerto (REFACTOR)

- [ ] 10.1 Retirar `session_summary` de `BuiltContext` y de los dos puntos de inyección `[Session summary]` en `agent.rs`; eliminar los `session_summary: None` de las ramas de `build()`. Ejecutar `cargo test --lib orchestrator` y confirmar GREEN.
- [ ] 10.2 Eliminar la rama `[tags]` de `format_memory` y el resto del camino JSON + coseno en Rust (`cosine_similarity` y el parseo de `embedding`), si ya no tiene otros usos. Ejecutar `cargo test --lib orchestrator::context_builder && cargo test --lib db::repos::memory` y confirmar GREEN.
- [ ] 10.3 Ejecutar `cargo fmt --check`, `cargo clippy -- -D warnings` y `cargo test`; confirmar todo verde.
- [ ] 10.4 Ejecutar `cd frontend && npx tsc --noEmit && npm run build`; confirmar verde.

## 11. Reconstrucción del índice desde la fuente (sin backfill)

- [ ] 11.1 Implementar el reseteo de la fuente: `UPDATE messages SET is_indexed = 0, summary_ref = NULL` y vaciado de `memory` y `vec_memory`. Los mensajes son el original y las fichas son datos derivados: es más robusto re-archivar que re-embeder contenido ya resumido por un LLM. Verificar en un test de integración que tras el reseteo el worker vuelve a archivar. Ejecutar `cargo test --test migrations` y confirmar GREEN.
- [ ] 11.2 Ejecutar la reconstrucción sobre la BD de trabajo: en local `cargo run --bin valet-reindex` y esperar a que el `EpisodicMemoryWorker` rearchive todo con el backend `vec0`. Verificar el conteo final: `SELECT COUNT(*) FROM memory` y `SELECT COUNT(*) FROM vec_memory` alineados por id.
- [ ] 11.3 Confirmar que no queda ningún embedding inbuscable (imposible por el esquema `vec0`) y que no existe ningún vector huérfano de otra dimensión.

## 12. Verificación final

- [ ] 12.1 Ejecutar `just check-spec` y confirmar que el change proposal está activo y aprobado.
- [ ] 12.2 Ejecutar `openspec validate episodic-memory-injection --strict`; confirmar válido.
- [ ] 12.3 Ejecutar `git status --porcelain`; confirmar que solo aparece el directorio `openspec/changes/episodic-memory-injection/` (y los ficheros de código del change, si ya se ha implementado).
- [ ] 12.4 Ejecutar `grep -rn -i "ANN" openspec/changes/episodic-memory-injection/ | grep -vi -e "no es" -e "Sin ANN" -e "ni un indice" -e "no lo es" -e "falso"`; confirmar que no devuelve ninguna afirmación de que sqlite-vec sea ANN.
