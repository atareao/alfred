# Change Proposal: f3-frontend-chat

## Why
Alfred es un asistente personal, no un foro de múltiples hilos. Tiene que sentirse como hablar con una persona: un único chat persistente que guarda todo el historial. Los chats efímeros son para consultas puntuales (ej. "tiempo en Nardó") que aparecen como una conversación temporal y se descartan.

## What Changes
Se implementa una UI de chat único con sidebar minimalista: el chat principal siempre visible y, opcionalmente, chats efímeros como items adicionales. Sin CRUD de conversaciones en el frontend (la API existe para cuando el orquestador la necesite).

## Scope

### Incluye
- API client tipado (conversaciones, mensajes, perfil)
- Hook `useMainChat` — carga la conversación principal (única, por defecto)
- Hook `useEphemeralChat` — para chats temporales puntuales
- Hook `useProfile` — get/update perfil
- Sidebar minimalista: Main Chat (fijo) + Ephemeral Chats (temporales) + Perfil
- ChatView + MessageBubble + MessageInput
- ProfileEditor como Drawer
- Tests de componentes

### Excluye
- CRUD de conversaciones en UI (la API existe, pero el frontend no expone crear/borrar conversaciones)
- Streaming SSE (Fase 5a)
- Búsqueda (Fase 4)
- MemoryExplorer (Fase 4)

## Impacto
- Simplifica drásticamente la UI: no hay lista de conversaciones editable
- Reduce hooks: useConversations desaparece, se reemplaza por useMainChat
- El backend ya soporta múltiples conversaciones (F2) — el frontend solo usa una por defecto
- Tests legacy de F1 + F2 deben seguir pasando

## Spec Deltas
- `specs/api/spec.md`: API client simplificado
- `specs/hooks/spec.md`: useMainChat, useEphemeralChat, useProfile
- `specs/components/spec.md`: Sidebar minimalista, ChatView único

## Risk Assessment
- **Chat efímero**: Debe ser fácil de crear y descartar sin contaminar el chat principal