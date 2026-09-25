# Limit tool call retries to 3 per ReAct loop

## Why
When a tool fails (e.g. Overpass timeout), the LLM receives the error and often tries to call the same tool again. This creates an infinite retry loop until `max_iterations` (10) is hit. The user gets a "Max iterations exceeded" error instead of a graceful response. Each failed retry wastes LLM tokens and time.

## What Changes
In `src/orchestrator/agent.rs`, add a per-tool call counter in the ReAct loop. Before executing a tool, check if it has already been called 3 times. If so, skip execution and send a message to the LLM telling it the tool is exhausted and to suggest alternatives.

## Scope
- `src/orchestrator/agent.rs` — add `tool_call_counts: HashMap<String, usize>` in both `process_message()` and `process_message_stream()`

## Impact
- **Users**: Instead of "Max iterations exceeded", the LLM will apologize and suggest alternatives after 3 failed attempts
- **Performance**: Saves wasted LLM tokens from repeated tool failures