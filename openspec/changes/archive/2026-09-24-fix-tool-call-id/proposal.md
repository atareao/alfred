# Fix: Incluir tool_call_id en mensajes tool

## Why

OpenRouter (API compatible con OpenAI) exige que los mensajes con `role: "tool"` incluyan un campo `tool_call_id` que coincida con el `id` del tool call del assistant message. Sin él, devuelve:

```
HTTP 400 — messages[16]: tool messages must include a non-empty string tool_call_id
```

## What Changes

| # | Tarea | Archivos | Grupo |
|---|-------|----------|-------|
| 1 | Añadir `tool_call_id` a `ChatMessage` | `src/llm/provider.rs` | A |
| 2 | Serializar `tool_call_id` en request a OpenRouter | `src/llm/openrouter.rs` | A |
| 3 | Pasar `tool_call_id` al construir tool result messages | `src/orchestrator/agent.rs` | A |

### provider.rs

Añadir campo a `ChatMessage`:
```rust
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}
```

### openrouter.rs

Al serializar mensajes con `role: "tool"`, incluir `tool_call_id`:
```rust
let mut msg = serde_json::json!({
    "role": m.role,
    "content": m.content,
});
if let Some(tool_call_id) = &m.tool_call_id {
    msg["tool_call_id"] = serde_json::json!(tool_call_id);
}
if let Some(tool_calls) = &m.tool_calls {
    msg["tool_calls"] = serde_json::json!(tool_calls.iter().map(...));
}
```

### agent.rs

Al construir tool result messages (3 lugares), incluir `tool_call_id` del tool call:
```rust
messages.push(ChatMessage {
    role: "tool".into(),
    content: serde_json::to_string(&result_value).unwrap_or_default(),
    tool_calls: None,
    tool_result: Some(result_value),
    tool_call_id: Some(tc.id.clone()),  // ← nuevo
});
```

## Impact

- **provider.rs**: nuevo campo opcional en `ChatMessage`, cambio trivial
- **openrouter.rs**: serializa `tool_call_id` cuando presente, no afecta a otros roles
- **agent.rs**: pasar `tc.id` en los 3 sitios donde se crean tool result messages
- Tests: actualizar creación de `ChatMessage` en tests (el nuevo campo es opcional con `skip_serializing_if`)