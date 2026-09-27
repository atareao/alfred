# Conflict Detector — DateTime Parsing Fix

## Contracts

### Function: `check_date`

```rust
pub async fn check_date(
    &self,
    profile_id: &str,
    date: &str,
) -> Result<Vec<ConflictAlert>, String>
```

### Change: Parse method for `start_time` / `end_time`

| Before (bug) | After (fix) |
|---|---|
| `NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")` | `DateTime::parse_from_rfc3339(s).map(|dt| dt.naive_utc())` |

### Error type

Both return `String` errors via `.map_err(|e| e.to_string())`.

## Scenarios

### Scenario 1: Happy path — RFC 3339 with trailing `Z`

- **Given** events with `start_time` = `"2026-09-24T10:00:00Z"` and `end_time` = `"2026-09-24T11:00:00Z"`
- **When** `check_date` is called
- **Then** parsing succeeds and comparison produces correct gap/overlap

### Scenario 2: No trailing Z (backward compat)

- **Given** events with `start_time` = `"2026-09-24T10:00:00"` (no `Z`)
- **When** `check_date` is called
- **Then** parsing succeeds (RFC 3339 allows omitting `Z`)