## ADDED Requirements

### Requirement: Extend events table with new fields

Add category, all_day, rrule, and reminder_minutes_before to the events table.

**Contracts:**

```sql
-- Migration: 20260925000002_extend_events.sql
ALTER TABLE events ADD COLUMN category TEXT NOT NULL DEFAULT 'default';
ALTER TABLE events ADD COLUMN all_day INTEGER NOT NULL DEFAULT 0;
ALTER TABLE events ADD COLUMN rrule TEXT;  -- RRULE string para recurrencia
ALTER TABLE events ADD COLUMN reminder_minutes_before INTEGER;  -- minutos antes para notificar

CREATE INDEX IF NOT EXISTS idx_events_category ON events(category);
CREATE INDEX IF NOT EXISTS idx_events_start_time ON events(start_time);
```

```rust
// Struct extendido
pub struct Event {
    pub id: String,
    pub profile_id: String,
    pub title: String,
    pub description: Option<String>,
    pub start_time: String,
    pub end_time: String,
    pub location: Option<String>,
    pub scope: String,          // "shared" | "personal"
    pub category: String,       // "default" | "work" | "personal" | "health" | "birthday" | "holiday"
    pub all_day: bool,
    pub rrule: Option<String>,  // "FREQ=WEEKLY;BYDAY=MO,WE,FR" | "FREQ=DAILY" | "FREQ=MONTHLY"
    pub reminder_minutes_before: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}
```

**Scenarios:**

#### Scenario: Create event with category and all_day
Given the events table has the new fields
When `create_event` is called with category "work" and all_day true
Then the event is stored with category "work" and all_day = 1

#### Scenario: Create recurring event with rrule
Given the events table has the rrule field
When `create_event` is called with title "Daily standup" and rrule "FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR"
Then the event is stored with the rrule
And the start_time/end_time represent Monday at 09:00-09:15

#### Scenario: Create event with reminder
Given the events table has reminder_minutes_before
When `create_event` is called with title "Doctor" and reminder_minutes_before = 60
Then the event is stored with reminder_minutes_before = 60

### Requirement: EventsRepo::delete method

Add soft-delete or hard-delete for events.

**Contracts:**

```rust
impl EventsRepo {
    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error>;
    pub async fn list_by_category(pool: &SqlitePool, profile_id: &str, category: &str) -> Result<Vec<Event>, sqlx::Error>;
}
```

**Scenarios:**

#### Scenario: Delete existing event
Given an existing event with id "evt-1"
When `EventsRepo::delete` is called with id "evt-1"
Then the event is removed from the events table

#### Scenario: Delete non-existent event
Given no event with id "nonexistent"
When `EventsRepo::delete` is called with id "nonexistent"
Then it succeeds (no-op) without error

#### Scenario: List events by category
Given events with categories "work" and "personal"
When `EventsRepo::list_by_category` is called with category "work"
Then only work events are returned

### Requirement: Expand events list_by_date_range to support recurring events

Recurring events (those with a non-null rrule) should be expanded when queried within a date range.

**Contracts:**

```rust
// Se modifica EventsRepo::list_by_date_range para expandir recurrencias.
// Por cada evento con rrule, se generan las instancias que caen dentro del rango.
// La expansion cubre: FREQ=DAILY, FREQ=WEEKLY, FREQ=MONTHLY
// Año y límite de expansión: máximo 365 instancias por evento.

fn expand_recurring(
    base_start: &str,
    base_end: &str,
    rrule: &str,
    range_start: &str,
    range_end: &str,
) -> Vec<(String, String)>;
```

**Scenarios:**

#### Scenario: List recurring event within range
Given an event "Standup" with rrule "FREQ=WEEKLY;BYDAY=MO,WE,FR" starting Monday
When `list_by_date_range` is queried for a week containing that Monday
Then the event appears 3 times (Mon, Wed, Fri)

#### Scenario: Recurring event outside range returns nothing
Given an event with rrule "FREQ=DAILY" but base dates are a year ago
When `list_by_date_range` is queried for next week
Then the event does NOT appear in results

#### Scenario: Non-recurring event still listed normally
Given a non-recurring event on Tuesday
When `list_by_date_range` is queried for that week
Then the event appears once