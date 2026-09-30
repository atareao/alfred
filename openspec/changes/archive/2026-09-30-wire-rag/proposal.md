# Proposal: Conectar el pipeline RAG (wire-rag)

## Why

El RAG de Valet está desconectado de punta a punta. La investigación previa a este change detectó cinco problemas encadenados:

1. **`ContextBuilder` nunca recibe pool ni provider.** En `src/lib.rs` se construye con `ContextBuilder::new()` (`pool: None, provider: None`) y solo se le asigna `rag_budget_tokens`. `build_rag_memories()` cae siempre en `fallback_memories()`, que devuelve `["memory1", "memory2"]` hardcodeado: el LLM recibe memorias falsas.
2. **`MemoryRepo::search_by_vector` nunca se ejecuta en producción.** Su único caller es `ContextBuilder`, que nunca tiene pool.
3. **Dos vías de embedding duplicadas.** `src/embeddings/` (módulo dedicado, hoy muerto) y `LLMProvider::embed` (vivo, usado por el worker episódico). El worker usa `LLMProvider::embed`, que en OpenRouter tiene el modelo hardcodeado (`openai/text-embedding-3-small`) y en Ollama usa el modelo de **chat** (`llama3.2:3b`) — un bug.
4. **No hay configuración de embeddings.** Ni `EMBEDDING_PROVIDER` ni `EMBEDDING_MODEL` existen en `Config` ni en `.env.example`.
5. **Fallo silencioso por dimensión.** `cosine_similarity` devuelve `0.0` cuando las longitudes difieren, sin log. Si el indexador y la consulta usan modelos distintos, el RAG devuelve vacío sin avisar.

Resultado: la memoria episódica se indexa con un modelo y el RAG (si estuviera conectado) consultaría con otro, devolviendo siempre vacío en silencio.

## What Changes

### 1. Fuente única de embeddings: `src/embeddings/`

- `EmbeddingConfig` se construye desde `Config` (env), sin modelo por defecto.
- `create_provider` pasa a devolver `Option<Box<dyn EmbeddingProvider>>` (`None` si no está configurado).
- Se elimina `LLMProvider::embed` del trait y de todos sus impls (OpenRouter, Ollama, Fallback) y de los mocks de test.
- `EpisodicMemoryWorker` pasa a recibir `Arc<dyn EmbeddingProvider>` y lo usa en `persist()`.
- `WorkerPool::start` recibe el provider de embeddings.
- `ContextBuilder` se cablea en `lib.rs` con `pool: Some(...)` y `provider: Some(...)`.

### 2. Embeddings configurables, sin default fijo

- `Config`: `embedding_provider` (`EMBEDDING_PROVIDER`), `embedding_model` (`EMBEDDING_MODEL`) y `embedding_dimension` (`EMBEDDING_DIMENSION`, opcional).
- Si no se configura provider + modelo → RAG deshabilitado con warning; `ContextBuilder.provider = None`; el worker episódico no arranca.
- `.env.example` documenta las nuevas variables.

### 3. Sin memorias placeholder

- Se elimina `fallback_memories()`. Si falta pool/provider o falla la búsqueda → `Vec::new()` + warning. Nunca se inyectan memorias falsas al LLM.

### 4. Seguridad de dimensión

- `search_by_vector` loguea un warning cuando descarta embeddings cuya dimensión difiere de la consulta (en vez de `0.0` silencioso).
- `EMBEDDING_DIMENSION` (opcional) valida las dimensiones generadas durante el re-indexado.

### 5. Re-indexado

- Nuevo binario `valet-reindex` (`src/bin/reindex.rs`) + `src/embeddings/reindex.rs` con `reindex_all(pool, provider)`.
- Regenera todos los embeddings de `memory` en `vec_memory` con el modelo configurado.

## Capabilities

### New Capabilities

- `embeddings`: configuración por entorno, `create_provider`, validación de dimensión y re-indexado.

### Modified Capabilities

- `orchestrator`: `ContextBuilder` cableado con pool + provider; sin memorias placeholder.
- `workers`: `EpisodicMemoryWorker` usa `EmbeddingProvider`.
- `llm/provider`: `LLMProvider` deja de exponer `embed`.
- `llm/openrouter`: se retira el `embed()` de `LLMProvider` (el de `embeddings::OpenRouterProvider` se mantiene).
- `db/repos`: `search_by_vector` avisa de dimensiones incompatibles.

## Impact

- **Editados**: `src/config.rs`, `src/embeddings/mod.rs`, `src/embeddings/provider.rs`, `src/embeddings/reindex.rs` (nuevo), `src/bin/reindex.rs` (nuevo), `Cargo.toml`, `src/lib.rs`, `src/orchestrator/context_builder.rs`, `src/workers/episodic_memory.rs`, `src/workers/pool.rs`, `src/llm/provider.rs`, `src/llm/openrouter.rs`, `src/llm/ollama.rs`, `src/llm/fallback.rs`, `src/db/repos/memory.rs`, `.env.example`, y los mocks de test que implementan `LLMProvider::embed`.
- **Specs**: `embeddings` (nueva), `orchestrator`, `workers`, `llm/provider`, `llm/openrouter`, `db/repos`.
- **Sin cambios** en la API pública HTTP ni en el esquema de base de datos.

## Fuera de alcance

- `sqlite-vec` (feature `vec0`): sigue sin cargarse; la búsqueda es lineal en Rust.
- `memory.profile_id`: no se añade; el RAG es global.
- Migración automática de embeddings al arrancar: el re-indexado es un comando explícito.
