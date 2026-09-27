# LLM: OpenRouter SSE Streaming — Finish Reason Handling

## Contracts

### `parse_sse_event` behavior

| Input | Current behavior | Expected behavior |
|---|---|---|
| `delta: {}, finish_reason: "tool_calls"` | Returns `Ok(None)` — no event emitted | Returns `Ok(Some(StreamEvent::Done(...)))` with accumulated tool calls |
| `delta: {tool_calls: [...]}, finish_reason: "tool_calls"` (single tool call) | Returns `Ok(Some(StreamEvent::ToolCall(...)))` — only first tool call | Returns `Ok(Some(StreamEvent::Done(...)))` **with all** accumulated tool calls |
| `delta: {tool_calls: [...]}, finish_reason: "tool_calls"` (multiple tool calls) | Returns one `ToolCall` for the first, rest lost | Returns `Done` with **all** accumulated tool calls |
| `delta: {}, finish_reason: "stop"` | Returns `Ok(Some(StreamEvent::Done(...)))` | Same (unchanged) |

### `StreamAccumulator.finalize()` — new method

```rust
impl StreamAccumulator {
    /// Consume all partial tool calls and produce a Vec<ToolCall>.
    /// Returns None if no tool calls were accumulated.
    pub fn finalize(&mut self) -> Option<Vec<ToolCall>>;
}
```

### Application Identification Headers

```rust
const APP_NAME: &str = "Alfred";
const APP_URL: &str = "https://github.com/atareao/alfred";
```

Los métodos `chat()`, `chat_stream()` y `embed()` de `OpenRouterProvider` (en `src/llm/openrouter.rs`), así como `embed()` de `embeddings::OpenRouterProvider` (en `src/embeddings/openrouter.rs`), añaden los headers `HTTP-Referer` y `X-Title` a todas las peticiones HTTP a OpenRouter.

## Scenarios

### Scenario 1: finish_reason "tool_calls" without tool_calls in delta

**Given** an SSE line with `{"choices":[{"delta":{},"finish_reason":"tool_calls","index":0}]}`  
**When** `parse_sse_event` processes it  
**Then** it returns `Ok(Some(StreamEvent::Done(...)))`  
**And** the Done response contains `tool_calls: None`

### Scenario 2: finish_reason "tool_calls" with accumulated tool calls

**Given** a StreamAccumulator that has a partial tool call stored  
**And** an SSE line with `{"choices":[{"delta":{},"finish_reason":"tool_calls","index":0}]}`  
**When** `parse_sse_event` processes it  
**Then** it returns `Ok(Some(StreamEvent::Done(...)))`  
**And** the Done response contains `tool_calls: Some([...])` with the accumulated tool calls  
**And** the arguments string is parsed as JSON

### Scenario 3: finish_reason "tool_calls" with multiple tool calls

**Given** a StreamAccumulator that has 2 partial tool calls stored  
**And** an SSE line with `{"choices":[{"delta":{},"finish_reason":"tool_calls","index":0}]}`  
**When** `parse_sse_event` processes it  
**Then** it returns `Ok(Some(StreamEvent::Done(...)))`  
**And** the Done response contains `tool_calls: Some([tool_call_1, tool_call_2])`

### Scenario 4: finish_reason "stop" still works (regression guard)

**Given** an SSE line with `{"choices":[{"delta":{},"finish_reason":"stop","index":0}],"usage":{...}}`  
**When** `parse_sse_event` processes it  
**Then** it returns `Ok(Some(StreamEvent::Done(...)))`  
**And** the Done response contains usage information  
**And** this is unchanged from current behavior

### Scenario 5: chat() sends application identification headers

**Given** un `OpenRouterProvider` configurado  
**When** se llama a `chat()` con un `ChatRequest` válido  
**Then** la petición HTTP incluye los headers `HTTP-Referer: https://github.com/atareao/alfred` y `X-Title: Alfred`

### Scenario 6: chat_stream() sends application identification headers

**Given** un `OpenRouterProvider` configurado  
**When** se llama a `chat_stream()` con un `ChatRequest` válido  
**Then** la petición HTTP incluye los headers `HTTP-Referer` y `X-Title`

### Scenario 7: embed() (LLMProvider) sends application identification headers

**Given** un `OpenRouterProvider` configurado  
**When** se llama a `embed()` con un texto  
**Then** la petición HTTP incluye los headers `HTTP-Referer` y `X-Title`

### Scenario 8: embed() (EmbeddingProvider) sends application identification headers

**Given** un `embeddings::OpenRouterProvider` configurado  
**When** se llama a `embed()` con un texto  
**Then** la petición HTTP incluye los headers `HTTP-Referer` y `X-Title`
