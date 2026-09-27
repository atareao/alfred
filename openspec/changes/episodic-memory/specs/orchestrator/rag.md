# orchestrator Specification — RAG Context Builder

## Requirements

### Requirement: ContextBuilder SHALL use vec_memory for RAG

**Given** un `ContextBuilder.build()` con `ContextStrategy::RAG`  
**When** se construye el contexto  
**Then** SHALL generar embedding del mensaje del usuario  
**And** SHALL consultar `vec_memory` por similitud coseno  
**And** SHALL filtrar resultados sumando `tokens_count` hasta `BUDGET_TOKENS`  
**And** SHALL incluir las fichas en `rag_memories` del `BuiltContext`

#### Scenario: RAG returns relevant memories within budget
**Given** memorias en vec_memory con tokens_count [150, 200, 300]  
**When** `build(RAG, profile, "user message about rust")` con budget = 400  
**Then** `rag_memories` contiene 2 fichas (150+200 <= 400)  
**And** las fichas tienen la metadata y contenido original

#### Scenario: RAG with empty vec_memory
**Given** vec_memory vacía  
**When** `build(RAG, profile, "anything")`  
**Then** `rag_memories` = vec vacío

### Requirement: ContextBuilder SHALL fall back to sliding window

**Given** `ContextStrategy::SlidingWindow` o `Historical`  
**When** se construye el contexto  
**Then** `rag_memories` SHALL estar vacío  
**And** `session_summary` SHALL ser None  
**And** el comportamiento legacy SHALL mantenerse

#### Scenario: SlidingWindow no RAG
**Given** `ContextStrategy::SlidingWindow`  
**When** `build()`  
**Then** `rag_memories.is_empty()` = true

### Requirement: Config SHALL include memory parameters

**Given** `Config::from_env()`  
**When** se cargan variables de entorno  
**Then** `memory_batch_tokens` SHALL ser 2000 por defecto  
**And** `memory_inactivity_minutes` SHALL ser 30 por defecto  
**And** `memory_overlap` SHALL ser 2 por defecto  
**And** `memory_poll_interval_minutes` SHALL ser 30 por defecto  
**And** `memory_model` SHALL ser `"mistralai/mistral-small"` por defecto (modelo propio para archivar, independiente del modelo de chat)  
**And** `rag_budget_tokens` SHALL ser 2000 por defecto

#### Scenario: Default values
**Given** sin env vars específicas de memoria  
**When** `Config::from_env()`  
**Then** `memory_batch_tokens` = 2000  
**And** `memory_inactivity_minutes` = 30  
**And** `memory_overlap` = 2  
**And** `memory_poll_interval_minutes` = 30  
**And** `memory_model` = "mistralai/mistral-small"  
**And** `rag_budget_tokens` = 2000

#### Scenario: Custom env vars
**Given** `MEMORY_BATCH_TOKENS=5000`, `MEMORY_INACTIVITY_MINUTES=15`, `MEMORY_MODEL=google/gemini-2.0-flash-lite`, `RAG_BUDGET_TOKENS=4000`  
**When** `Config::from_env()`  
**Then** los valores reflejan las env vars