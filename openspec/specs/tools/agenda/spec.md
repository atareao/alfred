# tools/agenda Specification

## Purpose
TBD - created by archiving change complete-agenda. Update Purpose after archive.

## Requirements

### Requirement: Calendar tool — add delete_event, list_by_category, support new fields

Extender la tool `calendar` con nuevas operaciones y soporte para los campos añadidos (category, all_day, rrule, reminder).

**Contracts:**

```rust
// Nuevas operaciones en CalendarTool

async fn delete_event(&self, args: Value) -> Result<ToolResult, ToolError>;
async fn list_by_category(&self, args: Value) -> Result<ToolResult, ToolError>;
```

```json
// Schema de parámetros expandido
{
  "type": "object",
  "properties": {
    "operation": {
      "type": "string",
      "enum": [
        "get_events",
        "check_availability",
        "create_event",
        "update_event",
        "delete_event",
        "list_by_category"
      ]
    },
    "profile_id": { "type": "string" },
    "date": { "type": "string" },
    "duration": { "type": "integer" },
    "title": { "type": "string" },
    "start": { "type": "string" },
    "end": { "type": "string" },
    "location": { "type": "string" },
    "scope": { "type": "string", "enum": ["shared", "personal"] },
    "id": { "type": "string" },
    "description": { "type": "string" },
    "category": { "type": "string", "enum": ["default", "work", "personal", "health", "birthday", "holiday"] },
    "all_day": { "type": "boolean" },
    "rrule": { "type": "string" },
    "reminder_minutes_before": { "type": "integer" }
  },
  "required": ["operation"]
}
```

**Scenarios:**

#### Scenario: Delete event via tool
Given an existing event
When `delete_event` is called with the event's id
Then the event is removed from the database
And success is returned

#### Scenario: Create event with all_day flag
When `create_event` is called with title "Cumpleaños Ana", all_day true, category "birthday"
Then the event is created with all_day = true
And category = "birthday"

#### Scenario: Create recurring event
When `create_event` is called with title "Reunión equipo", rrule "FREQ=WEEKLY;BYDAY=MO", start/end on Monday 10:00-11:00
Then the event is created with the rrule stored

#### Scenario: Create event with reminder
When `create_event` is called with reminder_minutes_before = 30
Then the event is stored with the reminder value

#### Scenario: List events filtered by category
Given events with categories "work" and "personal"
When `list_by_category` is called with category "work"
Then only work-category events are returned

#### Scenario: Get events expands recurring events
Given a weekly recurring event "Standup" (rrule FREQ=WEEKLY;BYDAY=MO,WE,FR)
When `get_events` is called with a date range covering 2 weeks
Then the result contains 6 event instances (3 per week)

#### Scenario: Delete non-existent event returns success
When `delete_event` is called with a non-existent id
Then it still returns success (idempotent)

### Requirement: Calendar tool — change delete_event permission to ExplicitApproval

Deleting events is destructive — it should require confirmation.

```rust
fn permission(&self) -> Permission {
    // Operaciones de solo lectura: NoConfirm
    // create_event, update_event: Notify
    // delete_event: ExplicitApproval
}
```

**Scenarios:**

#### Scenario: Delete event requires explicit approval
When the LLM calls `delete_event`
Then the guardrail check returns `ExplicitApproval`
And the orchestrator pauses to ask the user

### Requirement: Calendar tool — update_event supports start_time/end_time

The `update_event` operation must accept and persist `start` and `end` parameters so the LLM can reschedule events.

**Contracts:**

```rust
// EventsRepo::update — new signature
pub async fn update(
    pool: &SqlitePool,
    id: &str,
    title: Option<&str>,
    description: Option<&str>,
    location: Option<&str>,
    category: Option<&str>,
    all_day: Option<bool>,
    rrule: Option<&str>,
    reminder_minutes_before: Option<i32>,
    start_time: Option<&str>,   // NEW
    end_time: Option<&str>,     // NEW
) -> Result<bool, sqlx::Error>;
```

```rust
// UpdateEventRequest — new fields
pub struct UpdateEventRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub category: Option<String>,
    pub all_day: Option<bool>,
    pub rrule: Option<String>,
    pub reminder_minutes_before: Option<i32>,
    pub start_time: Option<String>,   // NEW
    pub end_time: Option<String>,     // NEW
}
```

**Scenarios:**

#### Scenario: Update event date via tool
Given an existing event on 2026-10-03
When `update_event` is called with id, start "2026-10-10T12:00:00Z", end "2026-10-10T13:30:00Z"
Then the event's start_time and end_time are updated to the new values
And `get_events` for 2026-10-03 returns no events
And `get_events` for 2026-10-10 returns the event

#### Scenario: Update event without changing dates (backward compat)
Given an existing event
When `update_event` is called with id and title only (no start/end)
Then the event's start_time and end_time remain unchanged
And the title is updated

#### Scenario: Update event via HTTP API
Given an existing event
When PUT /events/:id is called with start_time and end_time in the JSON body
Then the event's start_time and end_time are updated
And the response includes the updated event
