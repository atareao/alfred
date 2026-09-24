# Fix: LLM Providers descartan los tool_calls

## Why

Cuando el usuario pide el clima o geolocalización, el LLM (OpenRouter u Ollama) responde con una **llamada a tool real** (JSON con `{"name": "geo", "arguments": {...}}`). Sin embargo, los providers del LLM **descartan por completo esas llamadas a tool**:

- `src/llm/openrouter.rs` (líneas 121-146): solo extrae `content` y siempre devuelve `tool_calls: None`. Nunca parsea `response_body["choices"][0]["message"]["tool_calls"]`.
- `src/llm/ollama.rs` (líneas 79-92): igual — solo extrae `content`, ignora `response_body["message"]["tool_calls"]`.

**Consecuencia observable:** el orquestador ve `has_tool_calls == false`, trata el texto del LLM como respuesta final, y el LLM "habla" de hacer la geocodificación en texto plano ("voy a geocodificar...") sin ejecutar nunca la tool. Además, cuando hay tool call el campo `content` suele ser `null`, y `.as_str().unwrap_or("")` devuelve `""`, dejando la respuesta vacía.

## What Changes

| # | Tarea | Archivos | Grupo |
|---|-------|----------|-------|
| 1 | Parsear `tool_calls` en OpenRouterProvider | `src/llm/openrouter.rs` | A |
| 2 | Parsear `tool_calls` en OllamaProvider | `src/llm/ollama.rs` | A |
| 3 | Tests unitarios de parsing de tool_calls | `src/llm/openrouter.rs`, `src/llm/ollama.rs` | A |

### openrouter.rs
- Parsear `response_body["choices"][0]["message"]["tool_calls"]` como array
- Cada tool call: `id`, `function.name`, `function.arguments` (string JSON → parsear a `Value`)
- Construir `Vec<ToolCall>` y devolverlo en `ChatResponse.message.tool_calls`
- Mantener `content` (puede ser `null` cuando hay tool calls → usar `unwrap_or("")`)

### ollama.rs
- Parsear `response_body["message"]["tool_calls"]` como array
- Formato Ollama: cada tool call tiene `function.name` y `function.arguments` (objeto JSON, no string)
- Construir `Vec<ToolCall>` y devolverlo en `ChatResponse.message.tool_calls`

## Impact

- **`src/llm/openrouter.rs`**: deja de perder los tool_calls del LLM
- **`src/llm/ollama.rs`**: deja de perder los tool_calls del LLM
- **Sin romper nada**: el orquestador (`agent.rs`) ya maneja `tool_calls` correctamente; solo faltaba que los providers los devolvieran
- **Sin cambios en frontend ni en tools**: las tools (weather, geo, etc.) ya están correctamente implementadas