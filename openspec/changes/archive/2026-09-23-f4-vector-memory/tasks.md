# Tasks — f4-vector-memory

## TDD Task Checklist

### Database
- [ ] **4.1** Integrar sqlite-vec (src/db/vector.rs)
- [ ] **4.2** Funciones store/search para message_embeddings y memory_embeddings
- [ ] **4.3** Triggers FTS5 + config

### Embeddings
- [ ] **4.4** Trait EmbeddingProvider + errores
- [ ] **4.4** OllamaProvider
- [ ] **4.4** OpenRouterProvider
- [ ] **4.4** Factory create_provider

### Services
- [ ] **4.5** EmbeddingWorker (index_message, index_memory)
- [ ] **4.6** MemoryConsolidator (extraer hechos → memorias)
- [ ] **4.7** SearchService (RRF híbrido)

### API
- [ ] **4.8** Endpoint GET /api/search
- [ ] **4.8** Handler search con parámetros

### Tests
- [ ] **4.9** Tests: search_service
- [ ] **4.9** Tests: api/search
- [ ] **4.9** Tests: embeddings

### Verification
- [ ] **4.10** `cargo build` pasa
- [ ] **4.10** `cargo test` pasa (legacy + nuevos)
- [ ] **4.10** `cargo clippy -- -D warnings` pasa