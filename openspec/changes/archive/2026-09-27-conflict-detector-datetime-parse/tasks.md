# Tasks — ConflictDetector datetime parse

- [x] Create change proposal `conflict-detector-datetime-parse`
- [x] **GREEN**: Fix `NaiveDateTime::parse_from_str` → `trim_end_matches('Z')` in `conflict_detector.rs`
- [x] Run `cargo test` (all ConflictDetector tests pass)
- [x] Run `cargo clippy -- -D warnings`
- [x] Archive change proposal