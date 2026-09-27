## ADDED Requirements

### Requirement: Calendar tool — accept `duration` as alternative to `end` in `create_event`

The `create_event` operation currently requires both `start` and `end`. The LLM naturally
uses `start` + `duration` (e.g., "cena a las 21:00 durante 2 horas"), which fails.

Accept `duration` (in minutes) as an alternative to `end`. If `end` is present, use it
(backward compatible). If `end` is absent but `duration` is present, compute
`end = start + duration` minutes.

**Contracts:**

```rust
// No signature changes — the tool already receives a flat JSON object.
// Logic change in CalendarTool::create_event:
// - If `end` is present: use it (unchanged)
// - Else if `duration` is present: compute end = start + duration.minutes()
// - Else: error "Missing end or duration"
```

**Scenarios:**

#### Scenario: Create event with start + duration (what the LLM does)
When `create_event` is called with `start: "2026-09-26T21:00:00Z"` and `duration: 120`
Then the event is created successfully
And the computed `end_time` equals `"2026-09-26T23:00:00Z"`

#### Scenario: Create event with start + end still works (backward compat)
When `create_event` is called with `start: "2026-09-26T21:00:00Z"` and `end: "2026-09-26T23:00:00Z"`
Then the event is created successfully
And `end_time` equals `"2026-09-26T23:00:00Z"` (unchanged)

#### Scenario: Create event without end or duration returns error
When `create_event` is called with only `start`
Then the tool returns `InvalidArguments("Missing end or duration")`

#### Scenario: Create event with duration over midnight boundary
When `create_event` is called with `start: "2026-09-26T23:00:00Z"` and `duration: 90`
Then the event is created successfully
And the computed `end_time` equals `"2026-09-27T00:30:00Z"` (next day)