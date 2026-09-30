# Tasks

## 1. Config — embeddings por entorno

- [x] 1.1 RED: test de `Config::from_env()` que verifique `embedding_provider`, `embedding_model` y `embedding_dimension` (valores custom y `None` por defecto).
- [x] 1.2 GREEN: añadir los tres campos a `Config` y leerlos en `from_env()` (`EMBEDDING_PROVIDER`, `EMBEDDING_MODEL`, `EMBEDDING_DIMENSION`), sin default de modelo.
- [x] 1.3 GREEN: documentar las variables en `.env.example`.

## 2. `src/embeddings/` — fuente única

- [x] 2.1 RED: test de `EmbeddingConfig::from_config()` → `None` sin provider/modelo; `Some` con provider+modelo; error si `openrouter` sin API key.
- [x] 2.2 GREEN: reescribir `EmbeddingConfig` (provider, model, ollama_url, openrouter_api_key) y `from_config(&Config)`.
- [x] 2.3 GREEN: `create_provider` devuelve `Option<Box<dyn EmbeddingProvider>>`; `None` + warning si no está configurado.
- [x] 2.4 REFACTOR: eliminar el `Default` con modelo hardcodeado de `EmbeddingConfig`.

## 3. Eliminar `LLMProvider::embed`

- [x] 3.1 RED: test que verifique que `LLMProvider` no declara `embed` (compilación) y que los providers no lo implementan.
- [x] 3.2 GREEN: quitar `embed` del trait `LLMProvider` (`src/llm/provider.rs`) y de `openrouter.rs`, `ollama.rs`, `fallback.rs`.
- [x] 3.3 GREEN: quitar `embed` de todos los mocks de test (`workers/collapse.rs`, `workers/pool.rs`, `workers/episodic_memory.rs`, `routes/stream.rs`, `orchestrator/agent.rs`).
- [x] 3.4 REFACTOR: limpiar imports y helpers de embedding huérfanos en `src/llm/`.

## 4. Worker episódico usa `EmbeddingProvider`

- [x] 4.1 RED: test de `persist()` que verifique que el embedding se genera vía `Arc<dyn EmbeddingProvider>` (mock) y no vía `LLMProvider`.
- [x] 4.2 GREEN: cambiar `EpisodicMemoryWorker::start`/`evaluate`/`persist` para recibir `Arc<dyn EmbeddingProvider>`.
- [x] 4.3 GREEN: `WorkerPool::start` recibe `Option<Arc<dyn EmbeddingProvider>>`; si es `None`, no arranca el worker episódico y loguea warning.
- [x] 4.4 REFACTOR: actualizar los tests del worker y del pool.

## 5. Cablear `ContextBuilder`

- [x] 5.1 RED: test de `ContextBuilder` con pool + provider mock que devuelva memorias reales formateadas.
- [x] 5.2 RED: test de que sin pool/provider `rag_memories` es vacío (no placeholders) y se loguea warning.
- [x] 5.3 GREEN: eliminar `fallback_memories()`; devolver `Vec::new()` + warning en los caminos de fallo.
- [x] 5.4 GREEN: en `lib.rs`, construir el provider de embeddings y asignar `context_builder.pool` + `context_builder.provider`.
- [x] 5.5 REFACTOR: actualizar los tests existentes que esperaban placeholders (`test_rag_without_pool`, `test_rag_with_pool_no_provider`, `test_rag_context_has_memories`).

## 6. Seguridad de dimensión

- [x] 6.1 RED: test de `search_by_vector` que verifique que un embedding de dimensión distinta se descarta y se loguea warning.
- [x] 6.2 GREEN: en `search_by_vector`, comparar longitudes y loguear warning con ambas dimensiones antes de descartar.
- [x] 6.3 GREEN: validar `EMBEDDING_DIMENSION` en el re-indexado (error si la dimensión generada no coincide).

## 7. Re-indexado

- [x] 7.1 RED: test de `reindex_all(pool, provider, dimension)` con mock: regenera N embeddings y los upsertea en `vec_memory`.
- [x] 7.2 RED: test de idempotencia (ejecutar dos veces deja el mismo resultado).
- [x] 7.3 GREEN: implementar `src/embeddings/reindex.rs` con `reindex_all` y `ReindexReport`.
- [x] 7.4 GREEN: crear `src/bin/reindex.rs` y registrar `[[bin]] name = "valet-reindex"` en `Cargo.toml`.
- [x] 7.5 GREEN: el binario falla con mensaje claro si los embeddings no están configurados.

## 8. Verificación

- [x] 8.1 `cargo fmt --check` limpio.
- [x] 8.2 `cargo clippy --all-targets -- -D warnings` limpio.
- [x] 8.3 `cargo test` verde.
- [x] 8.4 `openspec validate wire-rag --strict` válido.
- [x] 8.5 Limpieza manual de la prosa de `openspec/specs/llm/openrouter/spec.md`: retirar el `embed()` de `LLMProvider` del contrato de headers y el Scenario 7.

## 9. Hardening (hallazgos de revisión)

- [x] 9.1 `Debug` de `EmbeddingConfig` redacta `openrouter_api_key`.
- [x] 9.2 Warning específico cuando falta `OPENROUTER_API_KEY`.
- [x] 9.3 Modelo obligatorio en `OllamaProvider`/`OpenRouterProvider` (sin defaults hardcodeados).
- [x] 9.4 Re-indexado atómico (genera todo, valida dimensión, escribe en una transacción).
- [x] 9.5 Variante `EmbeddingError::Storage` para errores de sqlx.
- [x] 9.6 Eliminado el campo muerto `EmbeddingConfig.dimension`.
- [x] 9.7 Warning si `EMBEDDING_DIMENSION` no parsea.
- [x] 9.8 Test del cableado de producción (`ContextBuilder.pool`/`provider`).
