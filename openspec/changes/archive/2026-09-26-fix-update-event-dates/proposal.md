# Fix: update_event should support start_time/end_time

## Intent
The `update_event` operation in the calendar tool accepts `start` and `end` parameters from the LLM but never passes them to the database. The `EventsRepo::update` function and the HTTP `UpdateEventRequest` struct also lack `start_time`/`end_time` fields. This means the LLM cannot move an event to a different date/time using `update_event`.

## Scope
- `src/db/repos/events.rs` — `EventsRepo::update` signature and SQL
- `src/tools/calendar.rs` — `CalendarTool::update_event` argument passing
- `src/handlers/events.rs` — `UpdateEventRequest` struct and handler
- Tests in both `events.rs` and `calendar.rs`

## Impact
Low. Backward-compatible — all existing callers pass `None` for the new params and behavior is unchanged. The LLM gains the ability to reschedule events.