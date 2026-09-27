# Tasks

## Phase 1: Shared module extraction (REFACTOR)
- [ ] Extract `DIAS`, `MESES`, `momento_del_dia`, `format_browser_timestamp` to `src/tools/time_format.rs`
- [ ] Extract `reverse_geocode` to `src/tools/geo_utils.rs`
- [ ] Update `agent.rs` to import from shared modules
- [ ] Verify tests pass with extraction

## Phase 2: New tools (TDD)
- [ ] RED: Write tests for `get_current_time` tool
- [ ] GREEN: Implement `get_current_time` tool
- [ ] RED: Write tests for `get_current_location` tool
- [ ] GREEN: Implement `get_current_location` tool

## Phase 3: Registration & persistence
- [ ] Register both tools in `ToolRegistry` (lib.rs)
- [ ] Add modules to `tools/mod.rs`
- [ ] Persist `browser_context` into settings on stream receive

## Phase 4: Verification
- [ ] `cargo test` all pass
- [ ] `cargo clippy -- -D warnings` clean
- [ ] `cargo fmt --check` clean