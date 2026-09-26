# Tasks: fix-update-event-dates

## TDD Checklist

### Phase: RED — Write failing tests

- [x] `src/db/repos/events.rs`: Update `test_update_event` to verify start_time/end_time are updated when passed
- [x] `src/tools/calendar.rs`: Update `test_update_event` to verify start/end are persisted after update

### Phase: GREEN — Implement

- [x] `src/db/repos/events.rs`: Add `start_time: Option<&str>` and `end_time: Option<&str>` params to `EventsRepo::update`, update SQL with `COALESCE`
- [x] `src/tools/calendar.rs`: Pass `start` and `end` from args to `EventsRepo::update`
- [x] `src/handlers/events.rs`: Add `start_time` and `end_time` to `UpdateEventRequest`, pass to `EventsRepo::update`

### Phase: REFACTOR

- [x] `cargo clippy -- -D warnings` — no new warnings
- [x] `cargo fmt --check` — clean formatting
- [x] All tests pass (418+ existing + new)