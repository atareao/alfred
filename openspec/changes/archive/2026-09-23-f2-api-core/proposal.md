# Change Proposal: f2-api-core

## Why
La Fase 1 estableció el scaffolding del proyecto. Ahora necesitamos la API REST funcional que Alfred usará para gestionar conversaciones, mensajes, perfiles de usuario y herramientas. Sin esta capa, el frontend no puede operar ni el orquestador (Fase 5a) puede ejecutar tools.

## What Changes
Se implementan los endpoints REST completos con sus modelos de datos, repositorios SQLite, handlers y rutas Axum. Se añaden tests de integración para cada endpoint.

## Scope

### Incluye
- Modelos de dominio: Conversation, Message, Memory, Profile, Tool, Pagination
- Repositorios SQLite para cada modelo (CRUD)
- Endpoints REST completos:
  - `GET/POST /api/conversations`
  - `GET/PUT/DELETE /api/conversations/:id`
  - `GET/POST /api/conversations/:id/messages`
  - `GET /api/conversations/:id/messages/:msg_id`
  - `GET/PUT /api/profile`
  - `GET/POST /api/memories`
  - `DELETE /api/memories/:id`
  - `GET /api/tools`
  - `PUT /api/tools/:id/toggle`
- Tests de integración para todos los endpoints
- Paginación con cursor-based (messages) y offset-based (conversations, memories)

### Excluye
- Autenticación PocketID (Fase 5a)
- Streaming SSE (Fase 5a)
- Búsqueda semántica (Fase 4)
- Tools del orquestador (Fase 5b)

## Impacto
- Añade ~15 nuevos archivos Rust
- Modifica `src/lib.rs` para montar nuevas rutas
- Modifica `Cargo.toml` si se necesitan nuevas dependencias (ej. `axum-extra` para JSON)
- Los tests existentes de health check deben seguir pasando

## Spec Deltas
- `specs/models/spec.md`: Tipos Rust, serialización, validación
- `specs/repos/spec.md`: Repositorios SQLite, queries, transacciones
- `specs/api/spec.md`: Endpoints, handlers, routing, tests

## Risk Assessment
- **Paginación cursor-based**: Requiere ordenación consistente por created_at. Mitigación: índices SQLite en created_at.
- **UUIDs como PK**: Rendimiento aceptable para el volumen esperado. Mitigación: índices adicionales si es necesario.
- **Sin auth todavía**: Los endpoints son abiertos. Mitigación: documentado como deuda técnica para F5a.