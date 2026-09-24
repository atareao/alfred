# Data: Seed, Export, Docker

## Contracts

```rust
// ── Seed ─────────────────────────────────────────────────────────────────

// Binario que crea una base de datos de desarrollo con datos de ejemplo:
// - 2 perfiles (Ana, Luis)
// - Conversaciones de ejemplo
// - Eventos, tareas, notas, contactos, hábitos de prueba
// - Menú semanal de ejemplo
// - Lista de la compra con items

// ── Export ───────────────────────────────────────────────────────────────

// GET /api/export → JSON completo del hogar:
// {
//   "profiles": [...],
//   "conversations": [...],
//   "messages": [...],
//   "events": [...],
//   "tasks": [...],
//   "notes": [...],
//   "contacts": [...],
//   "reminders": [...],
//   "meal_plans": [...],
//   "shopping_list": [...],
//   "habits": [...],
//   "habit_logs": [...],
//   "memories": [...],
//   "tools": [...],
// }
```

## Scenarios

### Seed: creates development data
**Given** a fresh database  
**When** `cargo run --bin seed` is executed  
**Then** the database is populated with realistic test data

### Export: returns all data
**Given** a database with data  
**When** `GET /api/export` is called  
**Then** a JSON object with all tables is returned

### Export: empty database
**Given** an empty database  
**When** `GET /api/export` is called  
**Then** a JSON object with empty arrays is returned

### Docker: production build
**Given** the Dockerfile  
**When** `docker compose -f docker-compose.prod.yml build` runs  
**Then** multi-stage images are built for backend and frontend