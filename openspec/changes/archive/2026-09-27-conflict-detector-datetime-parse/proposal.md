# Fix: ConflictDetector datetime parsing ("trailing input")

## Intent

Fix `ConflictDetector::check_date()` which fails at runtime with error "trailing input" because `NaiveDateTime::parse_from_str` with format `"%Y-%m-%dT%H:%M:%S"` cannot handle the trailing `Z` in RFC 3339 datetime strings stored in the database.

## Scope

Single file change: `src/workers/conflict_detector.rs`

## Impact

- Resolves runtime ERROR logs: `[ConflictDetector] Error: trailing input`
- Conflict detection now works correctly with RFC 3339 timestamps (e.g. `2026-09-24T10:00:00Z`)
- Existing tests remain unchanged and continue to pass