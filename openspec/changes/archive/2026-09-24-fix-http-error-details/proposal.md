# Fix: Incluir cuerpo de error HTTP en mensajes de error

## Why

Cuando OpenRouter devuelve un error HTTP (ej. 400 Bad Request), el error que llega al log es:

```
ERROR alfred::orchestrator::agent: ❌ Orchestrator error error=HTTP error: HTTP 400
```

Esto no da ninguna pista sobre **por qué** falló. OpenRouter (y otras APIs) devuelven un cuerpo JSON con el detalle del error, pero el provider lo descarta porque retorna el error antes de leer el body.

## What Changes

| # | Tarea | Archivos | Grupo |
|---|-------|----------|-------|
| 1 | Leer body en errores HTTP de OpenRouterProvider | `src/llm/openrouter.rs` | A |
| 2 | Leer body en errores HTTP de OllamaProvider | `src/llm/ollama.rs` | A |

### openrouter.rs (líneas 165-172)

Antes:
```rust
if !response.status().is_success() {
    let status = response.status().as_u16();
    return match status {
        429 => Err(LLMError::RateLimited { retry_after: 30 }),
        401 => Err(LLMError::AuthError("Invalid API key".into())),
        _ => Err(LLMError::HttpError(format!("HTTP {}", status))),
    };
}
```

Después:
```rust
if !response.status().is_success() {
    let status = response.status().as_u16();
    let body_text = response.text().await.unwrap_or_default();
    let details = if body_text.is_empty() {
        format!("HTTP {}", status)
    } else {
        // Try to extract a concise error message from JSON body
        if let Ok(body_json) = serde_json::from_str::<Value>(&body_text) {
            let msg = body_json["error"]["message"]
                .as_str()
                .or_else(|| body_json["error"].as_str())
                .unwrap_or(&body_text);
            format!("HTTP {} — {}", status, msg)
        } else {
            format!("HTTP {} — {}", status, body_text.trim())
        }
    };
    return match status {
        429 => Err(LLMError::RateLimited { retry_after: 30 }),
        401 => Err(LLMError::AuthError(format!("Invalid API key: {}", details))),
        _ => Err(LLMError::HttpError(details)),
    };
}
```

### ollama.rs (líneas 111-117)

Mismo patrón adaptado al formato de error de Ollama.

## Impact

- **openrouter.rs**: los errores HTTP ahora incluyen el mensaje del API (ej. "HTTP 400 — This model does not support tool calls with this API version")
- **ollama.rs**: misma mejora
- Tests existentes: sin cambios (los tests de error son de integración y usan mock HTTP)
- **Ninguna dependencia nueva**: `serde_json::Value` ya está importado