# F5a: Núcleo del Orquestador — Task Checklist

## LLM Providers
- [ ] 5a.1 Definir trait `LLMProvider` con chat, chat_stream, embed
- [ ] 5a.2 Implementar `OpenRouterProvider`
- [ ] 5a.3 Implementar `OllamaProvider`
- [ ] 5a.4 Implementar `FallbackProvider`

## Sistema de Tools
- [ ] 5a.5 Definir `Tool` trait + `ToolRegistry` + `Permission`

## Guardrails
- [ ] 5a.6 Implementar Guardrails: permisos + human-in-the-loop

## Contexto
- [ ] 5a.7 Clasificador de Contexto
- [ ] 5a.8 Constructor de Contexto (3 estrategias)
- [ ] 5a.9 Ventana Deslizante + Resumen de Sesión

## Orquestador
- [ ] 5a.10 Orquestador (ciclo ReAct completo)
- [ ] 5a.11 Analizador Segundo Plano (Reflexión)

## Streaming + API
- [ ] 5a.12 Streaming SSE desde el orquestador
- [ ] 5a.13 Endpoint POST /messages orquestado + approval endpoint

## Auth
- [ ] 5a.14 Auth PocketID + JWT + user_id en handlers

## Frontend
- [ ] 5a.15 Conectar frontend con SSE real

## Tests
- [ ] 5a.16 Tests de integración (chat, guardrails, contexto)