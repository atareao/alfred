## ADDED Requirements

### Requirement: Calendar tool (Agenda)
CRUD de eventos con scope shared/personal, soporte para date ranges, búsqueda de disponibilidad.

**Contracts:**
```rust
// Tool: get_events
// Tool: check_availability
// Tool: create_event
// Tool: update_event

pub struct Event {
    pub id: String,
    pub profile_id: String,
    pub title: String,
    pub description: Option<String>,
    pub start_time: String,  // ISO 8601
    pub end_time: String,
    pub location: Option<String>,
    pub scope: String,       // "shared" | "personal"
    pub created_at: String,
    pub updated_at: String,
}
```

**Scenarios:**
#### Scenario: Create event with scope
Given a profile with id "profile-1"
When `create_event` is called with title, start, end, scope "shared"
Then an event is created in the events table
And it is visible to both profiles

#### Scenario: List events for a date range
Given 3 events in the database for tomorrow
When `get_events` is called with date range covering tomorrow
Then all 3 events are returned

#### Scenario: Check availability finds free slots
Given events from 09:00-10:00 and 11:00-12:00
When `check_availability` is called with duration 60min
Then a free slot at 10:00-11:00 is returned

#### Scenario: Update event changes fields
Given an existing event
When `update_event` is called with new title
Then the event title is updated in the database

---

### Requirement: Tasks tool
CRUD de tareas con scope, prioridad, proyecto, estado.

**Contracts:**
```rust
pub struct Task {
    pub id: String,
    pub profile_id: String,
    pub content: String,
    pub status: String,       // "pending" | "completed" | "cancelled"
    pub priority: String,     // "low" | "medium" | "high"
    pub project: Option<String>,
    pub due_date: Option<String>,
    pub scope: String,
    pub created_at: String,
    pub updated_at: String,
}
```

**Scenarios:**
#### Scenario: Create task with priority
When `add_task` is called with content "Comprar leche" and priority "high"
Then a task is created with pending status

#### Scenario: List tasks filters by project
Given tasks in projects "Casa" and "Trabajo"
When `list_tasks` is called with project "Casa"
Then only Casa tasks are returned

#### Scenario: Complete task
Given a pending task
When `complete_task` is called
Then the task status changes to "completed"

---

### Requirement: Reminders tool
Recordatorios con datetime exacto, soporte para dismiss/snooze.

**Contracts:**
```rust
pub struct Reminder {
    pub id: String,
    pub profile_id: String,
    pub text: String,
    pub datetime: String,   // ISO 8601
    pub status: String,     // "pending" | "dismissed" | "snoozed"
    pub created_at: String,
}
```

**Scenarios:**
#### Scenario: Set reminder
When `set_reminder` is called with text and datetime
Then a reminder is created with pending status

#### Scenario: Dismiss reminder
Given a pending reminder
When `dismiss_reminder` is called
Then the reminder status changes to "dismissed"

#### Scenario: Snooze reminder
Given a pending reminder
When `snooze_reminder` is called with 15 minutes
Then the reminder datetime is extended by 15 minutes

---

### Requirement: Knowledge/Notes tool
Notas unificadas con categorías (idea, journal, fact) y tags, indexadas en FTS5 + sqlite-vec.

**Contracts:**
```rust
pub struct Note {
    pub id: String,
    pub profile_id: String,
    pub content: String,
    pub category: String,   // "idea" | "journal" | "fact" | "todo"
    pub tags: Option<String>, // JSON array
    pub created_at: String,
    pub updated_at: String,
}
```

**Scenarios:**
#### Scenario: Create note with category
When `create_note` is called with content "Recordar comprar pan" and category "todo"
Then a note is created and indexed in FTS5

#### Scenario: List notes by category
Given notes in categories "idea" and "journal"
When `list_notes` is called with category "idea"
Then only idea notes are returned

---

### Requirement: Contacts tool
CRUD de contactos, scope personal, búsqueda por nombre/teléfono.

**Contracts:**
```rust
pub struct Contact {
    pub id: String,
    pub profile_id: String,
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
```

**Scenarios:**
#### Scenario: Add contact
When `add_contact` is called with name and phone
Then a contact is created

#### Scenario: Search contacts
Given contacts "Ana García" and "Luis Pérez"
When `search_contacts` is called with "Ana"
Then only "Ana García" is returned

---

### Requirement: Unified Search tool
Busca en TODAS las dimensiones (eventos, tareas, notas, contactos, mensajes, memorias) vía FTS5.

**Contracts:**
```rust
// Tool: unified_search({ query, dimensions? })
```

**Scenarios:**
#### Scenario: Search across all dimensions
Given data in events, tasks, notes, and contacts tables
When `unified_search` is called with query "reunión"
Then results from multiple dimensions are returned