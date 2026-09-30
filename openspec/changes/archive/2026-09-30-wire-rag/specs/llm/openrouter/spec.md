# Spec Delta: llm/openrouter

## ADDED Requirements

### Requirement: OpenRouterProvider SHALL NOT implement embed on LLMProvider

`OpenRouterProvider` SHALL NOT implementar `embed` como parte de `LLMProvider`. La generación de embeddings de OpenRouter SHALL realizarse exclusivamente a través de `embeddings::OpenRouterProvider`, que mantiene los headers de identificación de la aplicación.

**Given** `OpenRouterProvider` en `src/llm/openrouter.rs`  
**When** se inspecciona su impl de `LLMProvider`  
**Then** SHALL NOT contener `async fn embed`  
**And** el embedding de OpenRouter SHALL generarse vía `embeddings::OpenRouterProvider`  
**And** `embeddings::OpenRouterProvider::embed()` SHALL seguir enviando los headers `HTTP-Referer` y `X-Title`

#### Scenario: El provider LLM no implementa embed
**Given** el impl `LLMProvider for OpenRouterProvider`  
**When** se inspecciona  
**Then** no contiene `async fn embed`

#### Scenario: El provider de embeddings mantiene los headers
**Given** un `embeddings::OpenRouterProvider` configurado  
**When** se llama a `embed()`  
**Then** la petición HTTP incluye `HTTP-Referer: https://github.com/atareao/valet-ai` y `X-Title: Valet`
