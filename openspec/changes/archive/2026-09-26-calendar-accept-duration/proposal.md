# Proposal: Calendar tool — accept `duration` as alternative to `end` in `create_event`

## Intent

The LLM naturally sends `start` + `duration` when creating calendar events (e.g., "cena a las 21:00 durante 2 horas"), but `create_event` currently requires `start` + `end`. This causes a tool call failure with `"Invalid arguments: Missing end"`.

## Scope

- `src/tools/calendar.rs`: `create_event` method
- Tool parameters schema (no changes needed — `duration` is already declared)

## Impact

Low. Backward compatible — `start` + `end` continues to work exactly as before. The only change is that `duration` (in minutes) is now accepted as a fallback when `end` is absent.

## Design

In `create_event`, after extracting `start`:

- If `end` is present, use it (current behavior, unchanged)
- If `end` is absent but `duration` is present, compute `end = start + duration minutes` using `chrono`
- If neither `end` nor `duration` is present, return `InvalidArguments("Missing end or duration")`

The computation uses `chrono::NaiveDateTime::parse_from_str` + `chrono::Duration::minutes` and formats back to RFC 3339.