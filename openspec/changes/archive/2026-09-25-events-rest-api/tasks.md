# Tasks — Events REST API

## TDD Checklist

### RED phase
- [x] Scenario 1: test_list_events_in_range — GET `/api/events?start=...&end=...` returns events
- [x] Scenario 2: test_list_events_empty_range — GET returns empty array
- [x] Scenario 3: test_list_events_missing_params — GET returns 422
- [x] Scenario 4: test_create_event — POST `/api/events` returns 201
- [x] Scenario 5: test_create_event_missing_fields — POST with `{}` returns 422
- [x] Scenario 6: test_update_event — PUT `/api/events/:id` returns 200
- [x] Scenario 7: test_update_event_not_found — PUT returns 404
- [x] Scenario 8: test_delete_event — DELETE `/api/events/:id` returns 204
- [x] Scenario 9: test_delete_event_not_found — DELETE idempotent returns 204

### GREEN phase
- [x] Create `src/handlers/events.rs` with all handlers
- [x] Create `src/routes/events.rs` with route definitions
- [x] Register in `src/routes/mod.rs`
- [x] Register in `src/lib.rs` `app_with_state()`
- [x] Add integration test file `tests/api/events.rs`
- [x] All tests pass (`cargo test`)
- [x] `cargo check` / `cargo clippy -- -D warnings` / `cargo fmt --check`

### REFACTOR phase
- [x] Clean up code, ensure idiomatic Rust
- [x] Remove any dead code or unused imports
- [x] Final run: `cargo test && cargo clippy -- -D warnings && cargo fmt --check`