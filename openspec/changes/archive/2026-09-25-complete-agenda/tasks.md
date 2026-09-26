# Tasks: Completar la Agenda

## T0 — Fix arquitectónico: inyección de profile_id

- [x] **T0.1: Eliminar profile_id del schema de calendar y reminders**
  - `calendar.rs` — quitar profile_id de parameters()
  - `reminders.rs` — quitar profile_id de parameters()
  - Test: ambas tools NO exponen profile_id en su schema

- [x] **T0.2: Inyectar profile_id en el orquestador**
  - En `process_message()` y `process_message_stream()`: antes de `registry.execute()`, insertar profile_id en tc.arguments
  - Test: mock LLM → tool call sin profile_id → orquestador lo inyecta → tool lo recibe

- [x] **T0.3: Mejorar descripciones**
  - calendar: "Agenda y calendario — eventos, citas, reuniones, cumpleaños, disponibilidad y huecos libres"
  - reminders: "Recordatorios — alarmas, avisos, alarmas temporales, posponer y descartar"

- [x] **T0.4: Tests de integración**
  - Test: "gestiona mi agenda" → inyecta profile_id → calendar se ejecuta
  - Test: "recuérdame algo" → inyecta profile_id → reminders se ejecuta

## T1 — Registrar tools en producción

- [x] **T1.1: Registrar CalendarTool**
  - `tool_registry.register(Box::new(crate::tools::calendar::CalendarTool::new(pool.clone())));`

- [x] **T1.2: Registrar RemindersTool**
  - `tool_registry.register(Box::new(crate::tools::reminders::RemindersTool::new(pool.clone())));`

## T2 — DB Layer (calendar)

- [x] **T2.1: Migration — Extend events table**
  - Crear `migrations/20260925000002_extend_events.sql`
  - category, all_day, rrule, reminder_minutes_before
  - Índices: idx_events_category, idx_events_start_time

- [x] **T2.2: EventsRepo — delete method**
  - Test: delete existing event
  - Test: delete non-existent is no-op

- [x] **T2.3: EventsRepo — list_by_category**
  - Test: filter returns only matching
  - Test: non-matching returns empty

- [x] **T2.4: EventsRepo — expand_recurring**
  - Función `expand_recurring` + modificar `list_by_date_range`
  - Test: weekly recurring 3x/semana
  - Test: daily recurring
  - Test: monthly recurring
  - Test: recurring fuera de rango no aparece
  - Test: non-recurring sigue listándose una vez

- [x] **T2.5: Event struct — nuevos campos**
  - category, all_day, rrule, reminder_minutes_before
  - Actualizar create, find_by_id, list, update

## T3 — Tool Layer (calendar)

- [x] **T3.1: create_event — nuevos campos**
  - category, all_day, rrule, reminder_minutes_before
  - Tests para cada uno

- [x] **T3.2: delete_event operation**
  - Test: delete existing event
  - Test: delete non-existent

- [x] **T3.3: list_by_category operation**
  - Test: list returns filtered

- [x] **T3.4: get_events — expansión de recurrencias**
  - Test: weekly event multiple times in range

- [x] **T3.5: delete_event — ExplicitApproval**
  - Test: permission check

## T4 — Frontend (calendar)

- [x] **T4.1: API client — Event endpoints**
- [x] **T4.2: Hooks — useEvents, useCreateEvent, useUpdateEvent, useDeleteEvent**
- [x] **T4.3: Component — CalendarView** (Ant Design Calendar)
- [x] **T4.4: Component — EventModal (crear/editar)**
- [x] **T4.5: Component — EventDetail (ver/eliminar)**
- [x] **T4.6: Tests frontend**

## T5 — Verificación

- [x] **T5.1: cargo test**
- [x] **T5.2: cargo clippy -- -D warnings**
- [x] **T5.3: cargo fmt --check**
- [x] **T5.4: npx tsc --noEmit**
- [x] **T5.5: npx vitest run**