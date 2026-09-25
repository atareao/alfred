# Fix Overpass API Timeouts and Parse Errors in GeoTool

## Why
The `GeoTool.search_places()` method has production bugs: requests to Overpass API hang indefinitely (no timeout), error responses don't include the raw body (impossible to debug), and the successful response is filtered (the LLM doesn't see full Overpass data).

## What Changes
1. Add 30s timeout to reqwest::Client
2. Read response body as raw text, check status before parsing JSON
3. Return full Overpass JSON body unfiltered on success
4. Include raw body text in error messages
5. Add `overpass_base_url` field for testability

## Scope
The `GeoTool.search_places()` method has three production bugs causing orchestrator failures:

1. **No HTTP timeout** on the `reqwest::Client` — requests to Overpass API can hang indefinitely (observed ~70s stalls in logs).
2. **No HTTP status check** before `resp.json()` — Overpass returns HTML error pages (429/502/504) which fail JSON deserialization with "error decoding response body".
3. **Incomplete regex escaping** — `overpass_escape()` only escapes `\` and `"`, but not regex special chars (`.`, `*`, `+`, `?`, `[`, `]`, `(`, `)`, `{`, `}`, `^`, `$`, `|`), causing malformed Overpass QL queries.

## Scope
- `src/tools/geo.rs` — the `GeoTool` struct and helper functions

## Impact
- **Users**: No more silent failures when asking "find cafes near me" or similar geo queries that hit Overpass
- **Operators**: No more stuck HTTP connections to overpass-api.de
- **Tests**: Existing unit tests (12) must remain green; new tests for timeout, status check, and regex escaping will be added