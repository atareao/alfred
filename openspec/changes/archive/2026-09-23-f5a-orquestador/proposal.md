# F5a: Núcleo del Orquestador

## Intención
Implementar el núcleo del orquestador de Alfred: LLM providers (OpenRouter + Ollama + fallback), sistema de tools (trait + registry + permission), guardrails con human-in-the-loop, clasificador de contexto (3 estrategias + override manual), constructor de contexto, ventana deslizante con resumen de sesión, ciclo ReAct completo, analizador de segundo plano (reflexión), streaming SSE, y autenticación PocketID + JWT.

## Alcance
- **Incluye:** 16 tareas definidas en PLAN.md (5a.1 a 5a.16)
- **Excluye:** Tools de dominio específico (F5b: agenda, tareas, notas, contactos, recordatorios) y tools de valor (F5c: clima, geo, comidas, hábitos, perfil multi-usuario). Workers proactivos (F6).
- **Dependencias:** F1-F4 completadas (scaffolding, API REST, frontend chat, memoria vectorial).

## Impacto
- **Nuevos módulos:** `src/llm/`, `src/orchestrator/`, `src/tools/`, `src/auth.rs`, `src/routes/stream.rs`, `src/handlers/stream.rs`
- **Modificaciones:** `src/main.rs` (registrar rutas, estado del orquestador), `src/state.rs` (añadir LLM provider, tool registry, orchestrator), `frontend/src/hooks/useSSE.ts` (conectar SSE real), `src/handlers/messages.rs` (integrar orquestador)
- **Tests nuevos:** `tests/api/chat.rs`, `tests/api/guardrails.rs`, `tests/api/context.rs`

## Specs incluidas
- `llm/provider` — LLMProvider trait + OpenRouter + Ollama + Fallback
- `tools/system` — Tool trait + Registry + Permission
- `orchestrator/agent` — ReAct loop + reflexión
- `orchestrator/context` — Classifier + Builder + Session Window
- `orchestrator/guardrails` — Permisos + HITL
- `stream` — SSE streaming endpoint
- `auth` — PocketID + JWT middleware