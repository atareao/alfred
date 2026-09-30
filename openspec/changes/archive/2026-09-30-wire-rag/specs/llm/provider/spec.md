# Spec Delta: llm/provider

## ADDED Requirements

### Requirement: LLMProvider SHALL NOT expose an embed method

El trait `LLMProvider` SHALL NOT declarar un método `embed`. La generación de embeddings SHALL gestionarse exclusivamente a través del trait `EmbeddingProvider` del módulo `embeddings`.

**Given** el trait `LLMProvider`  
**When** se define  
**Then** SHALL NOT declarar `embed`  
**And** los providers `OpenRouterProvider`, `OllamaProvider` y `FallbackProvider` SHALL NOT implementar `embed`  
**And** los embeddings SHALL gestionarse exclusivamente vía `EmbeddingProvider`

#### Scenario: El trait no declara embed
**Given** el código fuente de `src/llm/provider.rs`  
**When** se inspecciona el trait `LLMProvider`  
**Then** no contiene `async fn embed`

#### Scenario: Los providers no implementan embed
**Given** los impls de `LLMProvider` en `openrouter.rs`, `ollama.rs` y `fallback.rs`  
**When** se inspeccionan  
**Then** ninguno define `async fn embed`
