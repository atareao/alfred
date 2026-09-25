# Remove Multi-Conversation Complexity

## Intent
Alfred es un chat único e infinito. Sin embargo, el código actual maneja un modelo
multi-conversación (`conversations` table, `conversation_id` en messages, CRUD de
conversaciones, rutas REST con `:id` de conversación, sidebar de chats en frontend)
que es complejidad muerta: nunca hay más de una conversación.

## Scope
- **Backend Rust**: Eliminar tabla `conversations`, columna `conversation_id` de `messages`,
  modelo `Conversation`, repositorio `ConversationsRepo`, handlers y rutas de conversations.
  Simplificar `MessagesRepo`, handlers de messages, y el orchestrator para que no reciban
  `conversation_id`.
- **Frontend React**: Eliminar `useEphemeralChat`, sidebar de conversaciones, simplificar
  `useMainChat` y `useSSE` para que no usen `conversationId`. Simplificar `AppLayout` a
  un layout de una sola columna.
- **Tests**: Eliminar `tests/api/conversations.rs`, simplificar tests de messages, chat,
  y frontend. Actualizar tests existentes que referencian `conversation_id` o `Conversation`.
- **Schema**: Modificar migraciones directamente (no hay BD en producción que migrar).
  Un archivo `00001_initial.sql` reemplaza todos los migrations existentes.

## Impact
- **+** Elimina ~1500 líneas de código muerto entre backend y frontend
- **+** Simplifica drasticamente el modelo de datos y las APIs
- **+** Elimina la sidebar y toda la UI de gestión de conversaciones
- **+** El chat es conceptualmente un stream infinito, no una lista de conversaciones
- **-** Breaking change en todas las rutas REST (paths cambian)
- **-** Breaking change en frontend (layout completamente nuevo)