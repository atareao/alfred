## ADDED Requirements

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