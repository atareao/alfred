# Spec Delta: orchestrator

## ADDED Requirements

### Requirement: ContextBuilder SHALL be wired with the database pool and embedding provider

En producción, `ContextBuilder` SHALL construirse con `pool: Some(...)` y, cuando los embeddings estén configurados, `provider: Some(...)`, además de `rag_budget_tokens` leído de `RAG_BUDGET_TOKENS`.

**Given** `AppState::new_with_orchestrator()`  
**When** se construye el `ContextBuilder`  
**Then** `pool` SHALL ser `Some(...)`  
**And** `provider` SHALL ser `Some(...)` cuando `EMBEDDING_PROVIDER` y `EMBEDDING_MODEL` estén definidos  
**And** `rag_budget_tokens` SHALL leerse de `RAG_BUDGET_TOKENS`

#### Scenario: Builder de producción tiene pool
**Given** el arranque de producción  
**When** se inspecciona el `ContextBuilder`  
**Then** `pool` es `Some`

#### Scenario: Provider presente cuando hay configuración
**Given** `EMBEDDING_PROVIDER` y `EMBEDDING_MODEL` definidos  
**When** se construye el `ContextBuilder`  
**Then** `provider` es `Some`

#### Scenario: Provider ausente sin configuración
**Given** `EMBEDDING_PROVIDER` sin definir  
**When** se construye el `ContextBuilder`  
**Then** `provider` es `None`

### Requirement: RAG SHALL NOT inject placeholder memories

`ContextBuilder::build_rag_memories` SHALL devolver `Vec::new()` cuando falte el pool, falte el provider o falle la búsqueda, logueando un warning. SHALL NOT devolver memorias hardcodeadas.

**Given** un `ContextBuilder` sin pool o sin provider, o una búsqueda vectorial que falla  
**When** se construye el contexto con estrategia `RAG`  
**Then** `rag_memories` SHALL ser vacío  
**And** SHALL NOT contener valores hardcodeados como `"memory1"` o `"memory2"`  
**And** SHALL loguearse un warning

#### Scenario: Sin pool devuelve vacío
**Given** un `ContextBuilder` con `pool: None`  
**When** se construye el contexto `RAG`  
**Then** `rag_memories` es vacío

#### Scenario: Sin provider devuelve vacío
**Given** un `ContextBuilder` con `pool: Some` y `provider: None`  
**When** se construye el contexto `RAG`  
**Then** `rag_memories` es vacío

#### Scenario: Error de búsqueda devuelve vacío con warning
**Given** un `ContextBuilder` con pool y provider, y una búsqueda que falla  
**When** se construye el contexto `RAG`  
**Then** `rag_memories` es vacío  
**And** se loguea un warning

#### Scenario: Búsqueda real devuelve memorias formateadas
**Given** un `ContextBuilder` con pool y provider, y una fila en `memory` + `vec_memory`  
**When** se construye el contexto `RAG`  
**Then** `rag_memories` contiene la memoria formateada como `[{tags}] {content}`
