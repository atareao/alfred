# Handle tool execution errors gracefully in orchestrator

## Why
When a tool call fails (e.g. Overpass timeout, network error), the `registry.execute()` call returns `Err(ToolError)`. The `?` operator propagates this as `AgentError::ToolError`, which **breaks the entire ReAct loop** and sends an `SSEEvent::Error` to the frontend. The LLM never sees the error and cannot respond intelligently (apologize, suggest alternatives, retry with different params).

## What Changes
In `src/orchestrator/agent.rs`, `process_message_stream()`, wrap the `registry.execute()` call in a `match` expression instead of using `?`. When the tool returns `Err(e)`, convert it to `ToolResult { success: false, message: e.to_string() }` and let the existing `false` branch handle it — pushing the error message to the LLM so it can decide how to respond.

## Scope
- `src/orchestrator/agent.rs` — one match expression change

## Impact
- **Users**: Instead of a silent "Error" event, the LLM will apologize and suggest alternatives when a tool fails
- **Operators**: Errors are still logged; the LLM just gets a chance to respond gracefully