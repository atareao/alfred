# orchestrator Specification — RAG Context Builder

## Requirements

### Requirement: ContextBuilder SHALL have optional pool and provider fields

**Given** `ContextBuilder::new()` sin argumentos  
**When** se construye el builder  
**Then** `pool` SHALL ser `None`  
**And** `provider` SHALL ser `None`  
**And** `rag_budget_tokens` SHALL ser `2000`  
**And** la firma `fn new() -> Self` SHALL mantenerse sin cambios

### Requirement: ContextBuilder SHALL use vec_memory for RAG when configured

**Given** `ContextBuilder` con `pool = Some(pool)` y `provider = Some(provider)`  
**When** `build(RAG, profile_id, user_message)` es llamado  
**Then** SHALL generar embedding del `user_message` usando `provider.embed(user_message)`  
**And** SHALL consultar `MemoryRepo::search_by_vector(pool, &embedding, 10, rag_budget_tokens)`  
**And** SHALL incluir `memory.content` de cada resultado en `rag_memories`  
**And** SHALL actualizar `token_estimate` sumando `tokens_count` de las memorias

#### Scenario: RAG returns relevant memories within budget
**Given** memorias en vec_memory con tokens_count [150, 200, 300]  
**And** ContextBuilder con pool, provider y rag_budget_tokens = 400  
**When** `build(RAG, "profile-1", "user message about rust")`  
**Then** `rag_memories` contiene 2 entradas (150+200 <= 400)  
**And** NO contiene la ficha de 300 tokens (excede budget)

#### Scenario: RAG with empty vec_memory
**Given** vec_memory vacía  
**And** ContextBuilder con pool y provider configurados  
**When** `build(RAG, "profile-1", "anything")`  
**Then** `rag_memories` = vec vacío

### Requirement: ContextBuilder SHALL fall back to hardcoded memories without pool/provider

**Given** `ContextBuilder` con `pool = None` o `provider = None`  
**When** `build(RAG, profile_id, user_message)` es llamado  
**Then** SHALL mantener el comportamiento legacy  
**And** `rag_memories` SHALL contener `["memory1", "memory2"]`

#### Scenario: RAG sin pool cae a hardcoded
**Given** ContextBuilder con pool = None, provider = None  
**When** `build(RAG, "profile-1", "anything")`  
**Then** `rag_memories` = ["memory1", "memory2"]

### Requirement: Non-RAG strategies SHALL NOT query vec_memory

**Given** `ContextStrategy::SlidingWindow` o `Historical`  
**When** `build()` es llamado  
**Then** `rag_memories` SHALL estar vacío  
**And** NO se hace ninguna llamada a embedding o vec_memory

#### Scenario: SlidingWindow no RAG
**Given** `ContextStrategy::SlidingWindow`  
**When** `build()`  
**Then** `rag_memories` = vec vacío

#### Scenario: Historical no RAG
**Given** `ContextStrategy::Historical`  
**When** `build()`  
**Then** `rag_memories` = vec vacío