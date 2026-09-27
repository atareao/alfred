# Stats Recording Fix

## Why
Las stats muestran Total Calls = 27 pero todos los tokens = 0 y cost = $0.000000. Esto ocurre porque:
- El modelo se hardcodea como `"default"` en `ChatRequest` y `record_request` 
- El coste se hardcodea a `0.0` en `agent.rs`
- `cached_tokens` y `reasoning_tokens` se hardcodean a `0`
- `TokenUsage` no expone `cost`, `cached_tokens` ni `reasoning_tokens`

## What Changes
1. `TokenUsage` en `provider.rs`: añadir `cost: f64`, `cached_tokens: u32`, `reasoning_tokens: u32`
2. `OpenRouterProvider`: extraer `total_cost`, `cached_tokens` de la respuesta JSON
3. `OrchestratorConfig`: añadir `model: String`
4. `agent.rs`: usar `self.config.model` en vez de `"default"` y pasar tokens/coste reales a `record_request`
5. `lib.rs`: pasar `config.openrouter_model` al `OrchestratorConfig`

## Impacto
| Archivo | Cambio |
|---|---|
| `src/llm/provider.rs` | TokenUsage: +cost, +cached_tokens, +reasoning_tokens |
| `src/llm/openrouter.rs` | Extraer total_cost, cached_tokens del JSON response |
| `src/orchestrator/agent.rs` | OrchestratorConfig.model, usar en ChatRequest y record_request |
| `src/lib.rs` | Pasar model al OrchestratorConfig |