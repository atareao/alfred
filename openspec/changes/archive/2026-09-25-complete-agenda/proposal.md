# Proposal: Completar la Agenda (Calendar)

## Intent

Completar el sistema de agenda/calendario de Alfred para que sea funcionalmente completo para uso real. Actualmente:
1. El código de `CalendarTool` existe pero **nunca se registró en producción** en `lib.rs`
2. Todas las tools filtran `profile_id` al LLM, obligándole a pedírselo al usuario
3. La agenda tiene CRUD básico pero le faltan funcionalidades esenciales

## Scope

### Incluye
**T0 — Fix arquitectónico:** `profile_id` se elimina del schema expuesto por la tool y se inyecta desde el orquestador al ejecutar cada tool call. Esto evita que el LLM pida el profile_id al usuario. Afecta al orquestador y al schema de `calendar`.

**T1 — Registro:** `CalendarTool` se registra en `lib.rs` para que el LLM pueda usarla.

**T2-T5 — Agenda completa:**
- `delete_event` — Eliminación de eventos (tool + repo + tests)
- `all_day` — Eventos de día completo (cumpleaños, festivos)
- `category` — Categorías para filtrar y colorear (default, work, personal, health, birthday, holiday)
- Recurrencia — Eventos que se repiten (RRULE: diario, semanal, mensual)
- Notificaciones — Recordatorio de eventos (campo reminder_minutes_before)
- Frontend — Vista calendario con Ant Design Calendar + CRUD

### Excluye
- Tasks, reminders, contacts, knowledge, unified_search (ya existen en código, se registrarán en otra fase)
- Drag & drop para mover eventos
- CalDAV/Exchange
- Zona horaria configurable (todo UTC por ahora)

## Impacto

### Orquestador
```rust
// Antes de registry.execute() — inyectar profile_id
let mut args = tc.arguments.clone();
if let Some(obj) = args.as_object_mut() {
    obj.insert("profile_id".into(), serde_json::json!(profile_id));
}
let result = self.registry.execute(&tc.name, args).await?;
```

### Calendar tool
- `parameters()` ya no incluye `profile_id`
- Descripción actualizada: "Agenda y calendario — eventos, citas, reuniones, cumpleaños, disponibilidad y huecos libres"
- Nuevas operaciones: `delete_event`, `list_by_category`
- Nuevos campos: category, all_day, rrule, reminder_minutes_before
- `delete_event` requiere `ExplicitApproval`

### DB
```sql
ALTER TABLE events ADD COLUMN category TEXT NOT NULL DEFAULT 'default';
ALTER TABLE events ADD COLUMN all_day INTEGER NOT NULL DEFAULT 0;
ALTER TABLE events ADD COLUMN rrule TEXT;
ALTER TABLE events ADD COLUMN reminder_minutes_before INTEGER;
CREATE INDEX idx_events_category ON events(category);
CREATE INDEX idx_events_start_time ON events(start_time);
```

### Frontend
- CalendarView con Ant Design Calendar
- Colores por categoría
- Modal crear/editar con todos los campos
- Confirmación al eliminar

## Riesgos
| Riesgo | Mitigación |
|--------|-----------|
| Recurrencia compleja | Solo RRULE básico (diario, semanal, mensual), sin excepciones |
| Frontend calendario = cambio grande | Ant Design Calendar ya disponible en el proyecto |

## Archivos del proposal
```
openspec/changes/complete-agenda/
├── proposal.md
├── tasks.md
└── specs/
    ├── db/schema/spec.md
    ├── tools/agenda/spec.md
    ├── frontend/agenda/spec.md
    └── orchestrator/profile-injection/spec.md
```