# F5b: Tools Core

## Intención
Implementar las tools esenciales del día a día de Alfred: Agenda (eventos), Tareas, Recordatorios, Notas/Conocimiento, y Contactos. Cada tool sigue el `Tool` trait definido en F5a y opera sobre tablas SQLite con scope `shared`/`personal` para el modelo de dos perfiles.

## Alcance
- **Incluye:** 7 tareas del PLAN.md (5b.1 a 5b.7)
- **Excluye:** Tools de clima, geo, comidas, hábitos (F5c). Workers proactivos (F6).
- **Dependencias:** F1-F5a completadas (scaffolding, API, frontend, memoria vectorial, orquestador)

## Impacto
- **Nuevas tablas SQLite:** events, tasks, reminders, notes, contacts
- **Nuevos módulos Tool:** `src/tools/calendar.rs`, `tasks.rs`, `reminders.rs`, `knowledge.rs`, `contacts.rs`, `unified_search.rs`
- **Nuevos repos:** `src/db/repos/events.rs`, `tasks.rs`, `reminders.rs`, `notes.rs`, `contacts.rs`
- **Modificaciones:** `src/db/schema.rs` (nuevas tablas), `src/tools/mod.rs` (registrar módulos), `src/tools/registry.rs` (registrar tools en seed)