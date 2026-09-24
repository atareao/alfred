# Weather forecast fix + feedback visual + footer de tools

## Why

Tres problemas detectados en uso real:

1. **Clima**: al pedir pronóstico para "mañana", el LLM pasa "2026-09-25" (sin hora), el parseo a `DateTime<Utc>` falla, y `get_forecast` coge `list.first()` (hoy). Además, `extract_weather_data` busca `sys.sunrise/sunset` que no existe en forecast entries.

2. **Feedback**: el backend emite eventos `tool_call` y `tool_result` vía SSE, pero el frontend los ignora — nunca se muestra "Alfred está ejecutando weather..." al usuario.

3. **Footer**: no hay forma de saber qué tools se usaron en una respuesta. Queremos algo como:
```
---\n🔧 weather · 🔍 web_search
```

## What Changes

| # | Tarea | Archivos | Grupo |
|---|-------|----------|-------|
| 1 | Fix parseo de fechas en forecast | `src/tools/weather.rs` | A |
| 2 | Trackear tools usadas + footer en streaming | `src/orchestrator/agent.rs` | A |
| 3 | Mostrar tool calls en frontend durante streaming | `frontend/src/hooks/useMainChat.ts`, `frontend/src/components/ChatView.tsx` | B |
| 4 | Mostrar footer de tools usadas en la respuesta | `frontend/src/components/MessageBubble.tsx` | B |

### 1. weather.rs

- `get_forecast`: parsear date_str de forma más flexible:
  - "2026-09-25" → inicio de ese día (00:00:00Z)
  - "2026-09-25T12:00:00Z" → timestamp completo
  - Si falla el parseo, devolver error en lugar de coger `list.first()`
- `extract_weather_data`: hacer `sys` opcional (sunrise/sunset solo para current)

### 2. agent.rs (streaming path)

- Trackear nombres de tools ejecutadas en un `Vec<String>`
- Al finalizar, si hay tools usadas, añadir footer al `final_text`:
  `"\n\n---\n🔧 {}"` con las tools separadas por " · "

### 3. Frontend: feedback visual

- `useMainChat.ts`: pasar `onToolCall` a SSE para mostrar estado
- `ChatView.tsx`: mostrar indicador "🔧 Ejecutando tool..." durante streaming

### 4. Frontend: footer de tools

- Parsear el footer del markdown y mostrarlo estilizado

## Impact

- weather.rs: cambio en parseo de fechas, no rompe API
- agent.rs: solo afecta al streaming path, añade footer al texto
- Frontend: solo cambios visuales, no rompe nada