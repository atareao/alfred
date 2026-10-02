# Proposal: Inyección automática de memoria episódica

## Why

La memoria episódica de Valet se escribe pero no se lee. Las dos mitades del sistema están desconectadas:

- **Escritor**: `EpisodicMemoryWorker` (`src/workers/episodic_memory.rs`) agrupa los mensajes con `is_indexed = 0` (lote por `MEMORY_BATCH_TOKENS`, por defecto 2000 tokens, o por inactividad `MEMORY_POLL_INTERVAL_MINUTES`, por defecto 30 min), llama al LLM con `settings.archivist_prompt`, persiste una ficha en `memory` + su embedding en `vec_memory` y marca los mensajes con `is_indexed = 1` y `summary_ref`.
- **Lector**: en `src/orchestrator/agent.rs`, `ContextClassifier::classify()` decide una `ContextStrategy` (`!doc` → `RAG`, `!historico` → `Historical`, resto → `SlidingWindow`) y `ContextBuilder::build()` **solo** devuelve `rag_memories` no vacío en la rama `RAG`.

En la práctica esto significa que **el worker llena una base de datos que el agente no consulta**: la memoria episódica solo se inyecta si el usuario teclea `!doc`. En cualquier mensaje normal `rag_memories` es vacío; `Historical` y `SlidingWindow` también devuelven vacío. Y cuando la consulta `!doc` llega, el resultado arrastra un `[]` vacío: `format_memory()` (`src/orchestrator/context_builder.rs`) produce `"[{tags}] {content}"`, pero **nadie escribe `tags`** — el worker guarda `{"source":"episodic_worker","primary_message_ids":[...],"date_context":"..."}`. Verificado en la base de datos de producción: 7 fichas, **0 con `tags`**. El texto real inyectado es `[Memory context] [] - FECHA/CONTEXTO: ...`.

A esto se suman tres defectos estructurales:

1. **No hay umbral de similitud**: `MemoryRepo::search_by_vector` carga todas las filas, calcula el coseno en Rust, ordena y corta en `limit = 10` sin descartar puntuaciones malas. Entra el top-10 aunque el score sea pésimo.
2. **Hay un vector inbuscable**: la ficha más antigua (2026-09-29) tiene un embedding de 1536 dims frente a 6 de 1024. `search_by_vector` lo descarta con un `tracing::warn!` que nadie lee, así que esa ficha queda irrecuperable sin que el sistema lo advierta.
3. **El presupuesto de tokens no corta**: el filtro usa `continue`, así que se salta la ficha que no cabe y sigue buscando hueco entre las siguientes, metiendo una ficha **menos relevante por ser más pequeña**. Verificado con datos reales: con presupuesto 800 y fichas 347/307/248/245/210/164/138, `continue` da 3 fichas (347+307+138) y `break` daría 2 (347+307).

Medidas reales: ficha media **237 tokens** (rango 138–347); las 7 juntas 1659 tokens. Crecimiento **3,49 fichas/día**. Un vector de 1024 dims ocupa **12.810 caracteres (~11 KB)** en JSON.

## What Changes

- **BREAKING (interno) — la memoria deja de depender de la estrategia de contexto.** `ContextBuilder::build()` recupera memoria episódica en cada mensaje, con independencia de `ContextStrategy` (`SlidingWindow` y `Historical` inyectan igual). Desaparece el gating por estrategia.
- **BREAKING (interno) — desaparece el prefijo `[Memory context]`.** El bloque se compone **en código** (no como placeholder en `settings.system_prompt`, que es editable desde la UI y un borrado accidental desactivaría la memoria en silencio) y se envuelve en `<episodic_memory>` dentro de la sección `# CONTEXTO DE MEMORIA EPISÓDICA (CAPA B)`, con la instrucción explícita de que son antecedentes y no parte de la conversación actual. Se inyecta **antes** del historial de conversación. Ambos caminos (`process_message` y `process_message_stream`) cambian.
- **BREAKING (interno) — cambia el formato de ficha.** Se elimina el `[{tags}]` (nadie escribe `tags`: 7 fichas, 0 con `tags`). Cada ficha se precede de su fecha derivada de `memory.created_at` para dar cronología al modelo.
- **BREAKING (interno) — nuevo backend vectorial: sqlite-vec `vec0` sobre fuerza bruta.** Se activa el feature `vec0` de `Cargo.toml` (hoy declarado y sin usar) y se añade `libsqlite3-sys = "0.37"` como dependencia directa para registrar la extensión. Al arrancar se llama a `sqlite3_auto_extension(sqlite3_vec_init)`. **`vec0` no es ANN**: su búsqueda es un escaneo lineal por fuerza bruta, y a esta escala (3,49 fichas/día, más de 50 años para llegar a 10^5–10^6 vectores, donde un ANN real empezaría a compensar) es la elección correcta, no una limitación. Sin ANN, sin cuantización.
- **BREAKING (interno) — migración a tabla virtual `vec0`.** `vec_memory` pasa del esquema JSON `(id TEXT PRIMARY KEY, embedding TEXT)` a `CREATE VIRTUAL TABLE vec_memory USING vec0(id TEXT PRIMARY KEY, embedding float[1024] distance_metric=cosine)`, **con la columna `id` textual declarada explícitamente** (sin ella la tabla solo expone el `rowid` implícito y no se puede hacer el JOIN por id con `memory`). `memory` se conserva como fuente de verdad y el JOIN `memory m JOIN vec_memory v ON m.id = v.id` se mantiene.
- **Recuperación por pipeline explícito**, documentado paso a paso en `design.md`: embeber → KNN `vec0` (`MATCH … AND k = MEMORY_KNN_CANDIDATES ORDER BY distance`) → `similitud = 1 - distance` → **descartar por `SIMILARITY_THRESHOLD` sobre la similitud** → `final = similitud × exp(-ln2 × días / MEMORY_HALF_LIFE_DAYS)` → ordenar por `final` descendente → acumular tokens hasta `RAG_BUDGET_TOKENS` con `break`.
- **El corte por parecido se aplica a la similitud, nunca al resultado final.** Aplicarlo al resultado final hundiría las fichas viejas por debajo del corte y la memoria antigua quedaría inalcanzable, justo lo contrario de tener memoria. La antigüedad decide **quién va primero**, no **quién existe**.
- **El decaimiento se calcula en Rust, no en SQL.** La SQLite que enlaza `sqlx` no trae funciones matemáticas: `SELECT exp(-0.7)` falla con `no such function: exp` (y `pow`/`ln` tampoco). `julianday('now') - julianday(created_at)` da días, pero el producto se resuelve con `f64::exp()` en Rust.
- **Corrección del corte por presupuesto: `continue` → `break`.** Se deja de meter fichas menos relevantes por ser más pequeñas.
- **Cuatro mandos editables en caliente desde la UI**, en la tabla `settings` (como los prompts) y leídos en cada consulta, así que surten efecto sin reiniciar: `MEMORY_HALF_LIFE_DAYS = 90`, `SIMILARITY_THRESHOLD = 0.5` (provisional), `RAG_BUDGET_TOKENS = 800`, `MEMORY_KNN_CANDIDATES = 20`. `MEMORY_KNN_CANDIDATES` no limita lo que entra al prompt (eso lo hace el presupuesto) sino el margen de maniobra del reordenado por antigüedad. **No hay ritual de calibración**: al ser ajustables en caliente, los valores provisionales se afinan usándolos.
- **BREAKING (interno) — se borra `!doc` y con él toda la estrategia RAG**: la variante `ContextStrategy::RAG`, su rama en `ContextBuilder::build()`, `Override::Doc` del clasificador, y los tests `test_doc_override` y `test_classify_with_doc`. `!historico` y `!reset` siguen gobernando su propia estrategia, ya independiente de la memoria.
- **Deriva de dimensión: de visibilidad a imposibilidad estructural.** `vec0` declara `float[N]` y rechaza un vector de otra dimensión; el vector huérfano de 1536 dims deja de ser posible. Se retira el requisito `search_by_vector SHALL warn on embedding dimension mismatch`, que ya no puede darse. En su lugar se comprueba al arrancar que la dimensión declarada por la tabla coincide con `EMBEDDING_DIMENSION` y se falla de forma explícita si no.
- **Fail-fast si la extensión no carga.** Si `SELECT vec_version()` falla al arrancar, el sistema **no arranca**, con un error explícito. Un asistente que arranca aparentemente bien pero sin memoria es justo el fallo silencioso que este proyecto no tolera.
- **Reconstrucción del índice desde la fuente, sin backfill.** Los datos actuales son de prueba y se pueden perder. Los mensajes son el original y las fichas son derivados: se resetea `messages.is_indexed = 0` y `summary_ref = NULL`, se vacían `memory` y `vec_memory`, y el propio `EpisodicMemoryWorker` vuelve a archivar todo con el backend nuevo. Es más robusto que re-embeder contenido ya resumido por un LLM.
- **Limpieza de código muerto.** `session_summary` está siempre a `None` en las tres ramas de `build()`: el bloque `[Session summary]` es inalcanzable y se elimina. También se retira la rama `[tags]` de `format_memory`.

## Capabilities

Se eligen las capabilities existentes cuyo alcance encaja con cada delta; no se crea ninguna capability nueva.

### New Capabilities

<!-- Ninguna -->

### Modified Capabilities

- `orchestrator`: `ContextBuilder` recupera la memoria episódica por similitud, decaimiento y presupuesto, con independencia de la estrategia de contexto; se retiran `session_summary` y el formato `[{tags}]`; el requisito de placeholders pasa a ser agnóstico de estrategia.
- `orchestrator/agent`: el bloque de memoria se compone en código y se inyecta automáticamente en `process_message()` y `process_message_stream()` con las etiquetas `<episodic_memory>` y anclaje temporal por ficha.
- `db/repos`: `search_by_vector` resuelve la búsqueda KNN dentro de SQLite con `vec0`; se retira el aviso por dimensión incompatible (imposible con `vec0`); los cuatro mandos de memoria viven en `settings`.
- `db/schema`: la migración crea la tabla virtual `vec_memory USING vec0(id TEXT PRIMARY KEY, embedding float[N] distance_metric=cosine)`.
- `embeddings`: `reindex_all` escribe el embedding en la tabla virtual `vec0` en lugar de en una columna de texto JSON.
- `dependencies`: se añade `sqlite-vec` y `libsqlite3-sys` como dependencias directas, se registra la extensión con `sqlite3_auto_extension` y se falla al arrancar si no carga.
- `frontend`: el `SettingsDialog` incorpora un panel de ajustes de memoria con los cuatro mandos, persistidos en `settings`.

### Nota sobre `dependencies` frente a `app-state`

El registro de la extensión y el fail-fast encajan en `dependencies` y no en `app-state`: lo que se especifica es la relación del proyecto con dos dependencias Rust (`sqlite-vec` y `libsqlite3-sys`), la unificación de una única instancia de `libsqlite3-sys` con `sqlx-sqlite` y la garantía de que la extensión queda registrada en cada conexión. `app-state` describe el ciclo de vida del struct `AppState` (en concreto `shutdown_tx`), que no se toca en este change.

## Impact

- **Backend Rust**: `Cargo.toml` (activar feature `vec0`, añadir `libsqlite3-sys = "0.37"`), `src/lib.rs` (registro de `sqlite3_auto_extension` y fail-fast al arrancar; lectura en caliente de los cuatro mandos), `src/db/schema.rs` (migración a la tabla virtual `vec0` y siembra de los mandos en `settings`), `src/db/repos/memory.rs` (`search_by_vector` con `MATCH`/`k`/`ORDER BY distance`, sin parseo JSON ni coseno en Rust; `break` en el presupuesto), `src/workers/episodic_memory.rs` y `src/embeddings/reindex.rs` (escritura del vector binario en la tabla virtual), `src/orchestrator/agent.rs` (inyección en ambos caminos, `[Memory context]` → `<episodic_memory>`), `src/orchestrator/context_builder.rs` (`format_memory`, `session_summary`, decaimiento en Rust, selección por umbral/presupuesto), `src/orchestrator/context_classifier.rs` (eliminación de `Override::Doc` y `ContextStrategy::RAG`), `src/config.rs` (mandos de memoria y sus defaults).
- **Frontend**: panel "Memoria" del `SettingsDialog`, `useSettings.ts`, `types/index.ts`.
- **Tests**: `src/db/schema.rs`, `src/db/repos/memory.rs`, `src/workers/episodic_memory.rs`, `src/embeddings/reindex.rs`, `src/orchestrator/context_builder.rs`, `src/orchestrator/agent.rs`, `src/orchestrator/context_classifier.rs`, `src/lib.rs`, `tests/`, y `frontend/src/test/SettingsDialog.test.tsx`.
- **Datos**: los datos actuales son de prueba y se pueden perder. No hay backfill: el índice se reconstruye desde la fuente (`is_indexed = 0`, `summary_ref = NULL`, vaciado de `memory`/`vec_memory` y rearchivado por el worker).
- **API**: sin cambios en la API pública HTTP (`GET`/`PUT /settings` ya persisten claves arbitrarias, así que los cuatro mandos viajan por ahí sin rutas nuevas).
- **Instancia de tests**: ninguna aserción de test existente cambia salvo las tres excepciones declaradas (formato `[{tags}] {content}`, JSON en `vec_memory`, y los tests de `!doc` que desaparecen con la funcionalidad). Cualquier otro test que se rompa es una regresión, no una actualización.
