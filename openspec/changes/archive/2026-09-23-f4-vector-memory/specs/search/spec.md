# Search Spec — f4-vector-memory

## ADDED: EmbeddingWorker

```rust
// src/services/embedding_service.rs

pub struct EmbeddingWorker {
    provider: Box<dyn EmbeddingProvider>,
}

impl EmbeddingWorker {
    pub fn new(provider: Box<dyn EmbeddingProvider>) -> Self;
    
    /// Generate and store embedding for a message
    pub async fn index_message(&self, conn: &Connection, message: &Message) -> Result<()>;
    
    /// Generate and store embedding for a memory
    pub async fn index_memory(&self, conn: &Connection, memory: &Memory) -> Result<()>;
}
```

Comportamiento:
- Al crear un mensaje, generar embedding y guardarlo en `message_embeddings`
- Al crear una memoria, generar embedding y guardarlo en `memory_embeddings`
- Si sqlite-vec no está disponible, saltar silenciosamente
- Si el embedding falla (API down), loguear warning y continuar

## ADDED: MemoryConsolidator

```rust
// src/services/memory_service.rs

pub struct MemoryConsolidator {
    provider: Box<dyn EmbeddingProvider>,
}

impl MemoryConsolidator {
    pub fn new(provider: Box<dyn EmbeddingProvider>) -> Self;
    
    /// Extract facts from a message and store as memories
    pub async fn consolidate(&self, conn: &Connection, message: &Message) -> Result<Vec<Memory>>;
}
```

Comportamiento (simplificado para F4):
- Toma un mensaje del assistant, extrae líneas con hechos (p. ej. "Recuerda que...")
- Las guarda como memorias con categoría "fact"
- Genera embedding para cada memoria
- En F5a se integrará con el LLM para extracción semántica real

## ADDED: Hybrid Search (RRF)

```rust
// src/services/search_service.rs

pub struct SearchService {
    provider: Box<dyn EmbeddingProvider>,
}

impl SearchService {
    pub fn new(provider: Box<dyn EmbeddingProvider>) -> Self;
    
    /// Hybrid search: vector + FTS5 with Reciprocal Rank Fusion
    pub async fn search(
        &self,
        conn: &Connection,
        query: &str,
        search_type: SearchType,
        limit: i64,
    ) -> Result<Vec<SearchResult>>;
}

pub enum SearchType {
    Messages,
    Memories,
    All,
}

pub struct SearchResult {
    pub id: String,
    pub content: String,
    pub score: f64,
    pub source: String,  // "message" | "memory"
    pub created_at: String,
}
```

**RRF Algorithm:**
1. Run FTS5 search → get ranked results
2. Generate embedding for query → search vec0 → get ranked results
3. Combine with RRF: `score = 1/(k + rank_fts) + 1/(k + rank_vec)` where k=60
4. Sort by combined score, return top N

## ADDED: Search endpoint

`GET /api/search?q=<query>&type=message|memory|all&limit=20`

```rust
// src/handlers/search.rs
pub async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<PaginatedResponse<SearchResult>>, AppError>;
```

```json
// Response
{
  "data": [
    {
      "id": "msg-uuid",
      "content": "Texto del mensaje",
      "score": 0.89,
      "source": "message",
      "created_at": "2024-01-01T00:00:00Z"
    }
  ],
  "total": 5
}
```

## Scenarios (BDD)

### Scenario: FTS5 search returns results
- **Given** messages exist with the word "receta"
- **When** SearchService searches for "receta" with type=Messages
- **Then** returns messages containing "receta" ordered by relevance

### Scenario: Hybrid search returns vector results too
- **Given** messages with similar semantic content exist
- **When** SearchService searches with hybrid mode
- **Then** returns results from both FTS5 and vector search

### Scenario: Empty query returns error
- **Given** an empty query string
- **When** GET /api/search?q= is called
- **Then** returns 400 Bad Request

### Scenario: Search across all types
- **Given** both messages and memories exist
- **When** GET /api/search?q=test&type=all
- **Then** returns results from both tables with source field