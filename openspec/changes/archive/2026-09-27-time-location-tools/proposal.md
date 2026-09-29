# Change Proposal: Time and Location Tools for Valet

## Intent
Add two new tools (`get_current_time`, `get_current_location`) so Valet can answer "¿qué hora es?" and "¿dónde estoy?" with accurate, up-to-date information instead of guessing or complaining.

## Scope
- Extract `format_browser_timestamp`, `DIAS`, `MESES`, `momento_del_dia` from `src/orchestrator/agent.rs` into a shared module `src/tools/time_format.rs`
- Extract `reverse_geocode` from `src/orchestrator/agent.rs` into a shared module `src/tools/geo_utils.rs`
- Create `src/tools/current_time.rs` — `get_current_time` tool
- Create `src/tools/current_location.rs` — `get_current_location` tool
- Register both tools in `ToolRegistry`
- Persist `browser_context` (timezone, coordinates, location_name) into DB settings on receive
- Update `agent.rs` to import shared formatting functions

## Impact
- No breaking changes to existing tools or API
- BrowserContext injection continues working identically (same format, shared code)
- New tools can be enabled/disabled via the tools management UI