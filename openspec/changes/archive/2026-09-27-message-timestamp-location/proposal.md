# Message Timestamp & Location Display

## Intent
Mostrar fecha, hora y localización de forma no intrusiva en cada mensaje del chat,
usando los datos de geolocalización que el frontend ya envía (`browser_context`).

## Scope

### Incluye
- Añadir campo `location` al modelo `Message` (Rust + SQLite)
- Migración DB: columna `location TEXT` nullable en `messages`
- `MessagesRepo::create()` acepta y persiste `location` (opcional)
- API `/api/messages` devuelve `location` en cada mensaje
- Frontend TypeScript: añadir `location?: string | null` a `Message`
- Nuevo componente `DateSeparator` entre grupos de mensajes (mismo día)
- `MessageBubble`: mostrar timestamp (hora) + ubicación inline, no intrusivo
- Tests: backend (modelo, repo) + frontend (MessageBubble, DateSeparator)

### No incluye
- Edición manual de ubicación por mensaje
- Mostrar ubicación en tool results
- Geolocalización desde el backend (ya la recibe del frontend via `browser_context`)

## Impacto

| Área | Cambio |
|---|---|
| `src/models/message.rs` | Añadir `location: Option<String>` |
| `migrations/` | Nueva migración `20260927000003_message_location.sql` |
| `src/db/repos/messages.rs` | `create()` acepta `location`, queries devuelven columna |
| `src/handlers/messages.rs` | Pasa location desde settings al crear |
| `frontend/src/types/index.ts` | Añadir `location` a interface Message |
| `frontend/src/components/MessageBubble.tsx` | Mostrar timestamp + location inline |
| `frontend/src/components/DateSeparator.tsx` | Nuevo componente |
| `frontend/src/components/ChatView.tsx` | Integrar DateSeparator entre mensajes |
| Tests | Backend + frontend tests |