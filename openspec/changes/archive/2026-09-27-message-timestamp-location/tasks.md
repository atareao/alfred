# Tasks: message-timestamp-location

## Backend

- [ ] **T1**: Añadir `location: Option<String>` al struct `Message` en `src/models/message.rs`
  - Scenarios: Location se persiste, Location es null

- [ ] **T2**: Crear migración SQL `20260927000003_message_location.sql`
  - Scenarios: Migration añade columna location idempotentemente

- [ ] **T3**: Actualizar `MessagesRepo::create()` para aceptar y persistir `location`
  - Scenarios: Location se persiste, Location es null

- [ ] **T4**: Actualizar `find_by_id()` y `list_all()` queries para incluir columna `location`
  - Scenarios: LIST devuelve location, find_by_id devuelve location

- [ ] **T5**: Actualizar handler `create_message` para pasar location desde settings
  - Scenarios: Location se persiste

- [ ] **T6**: Migrar DB schema test (`src/db/schema.rs`) para incluir nueva columna
  - Scenarios: Migration añade columna location idempotentemente

## Frontend

- [ ] **F1**: Añadir `location?: string | null` a la interface `Message` en `frontend/src/types/index.ts`

- [ ] **F2**: Crear componente `DateSeparator` (Hoy / Ayer / fecha completa)
  - Scenarios: DateSeparator muestra "Hoy", "Ayer", fecha completa

- [ ] **F3**: Actualizar `ChatView` para intercalar `DateSeparator` entre grupos de mensajes
  - Scenarios: Misma fecha sin separador, Fecha diferente con separador

- [ ] **F4**: Actualizar `MessageBubble` para mostrar timestamp + location inline, no intrusivo
  - Scenarios: Timestamp formatea hora, Timestamp con location, Timestamp sin location, Streaming sin timestamp

## Tests

- [ ] **T7**: Tests unitarios backend (model + repo) para location
- [ ] **F5**: Tests frontend para DateSeparator
- [ ] **F6**: Tests frontend para MessageBubble con timestamp y location