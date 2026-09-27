# Tasks: Calendar tool — accept `duration` as alternative to `end`

## TDD Task Checklist

### RED phase

- [x] Spec written and approved
- [ ] Write test `test_create_event_with_duration` — LLM sends start + duration, verify end is computed
- [ ] Write test `test_create_event_with_start_end_backward_compat` — start + end still works unchanged
- [ ] Write test `test_create_event_missing_end_and_duration` — returns InvalidArguments
- [ ] Write test `test_create_event_duration_midnight_boundary` — duration crosses midnight
- [ ] Run `cargo test` and confirm new tests FAIL but existing tests remain GREEN

### GREEN phase

- [ ] Implement `duration` fallback in `CalendarTool::create_event`
- [ ] Run `cargo test` — all tests pass (100% GREEN)
- [ ] Run `cargo check` — no compilation errors

### REFACTOR phase

- [ ] Run `cargo clippy -- -D warnings` — zero warnings
- [ ] Run `cargo fmt --check` — proper formatting
- [ ] Run `cargo test` — all tests still pass

### Archive

- [ ] `openspec archive calendar-accept-duration`
- [ ] Clean up change directory