# Spec Delta: workers

## ADDED Requirements

### Requirement: EpisodicMemoryWorker SHALL use the EmbeddingProvider for embeddings

`EpisodicMemoryWorker` SHALL generar los embeddings de las fichas mediante un `Arc<dyn EmbeddingProvider>` inyectado, y SHALL NOT usar `LLMProvider::embed`. El mismo provider SHALL ser el usado por `ContextBuilder` para las consultas.

**Given** el `EpisodicMemoryWorker`  
**When** persiste una ficha de memoria  
**Then** SHALL generar el embedding vía `Arc<dyn EmbeddingProvider>`  
**And** SHALL NOT llamar a `LLMProvider::embed`  
**And** el provider SHALL ser el mismo que usa `ContextBuilder` para consultar

#### Scenario: persist usa EmbeddingProvider
**Given** un worker con un `EmbeddingProvider` mock  
**When** `persist()` guarda una ficha  
**Then** el mock registra la llamada a `embed`  
**And** el embedding se almacena en `vec_memory`

#### Scenario: Worker no arranca sin provider configurado
**Given** `WorkerPool::start` con `embedding_provider = None`  
**When** se construye el pool  
**Then** el worker episódico NO SHALL arrancar  
**And** SHALL loguearse un warning
