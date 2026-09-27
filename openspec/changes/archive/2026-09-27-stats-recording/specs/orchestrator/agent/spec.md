## ADDED Requirements

### Requirement: Orchestrator records stats after each LLM call
The Orchestrator SHALL call `StatsRepo::record_request()` after each LLM call in both `process_message()` and `process_message_stream()`.

#### Scenario: process_message records stats
- **WHEN** `process_message("profile-1", "hello")` completes one iteration
- **THEN** there is exactly one row in `llm_requests` with status = "success"

#### Scenario: process_message_stream records stats
- **WHEN** `process_message_stream()` receives `StreamEvent::Done`
- **THEN** there is at least one row in `llm_requests` with tokens

#### Scenario: LLM error records stats with error status
- **WHEN** a mock LLM that always fails is used with `process_message()`
- **THEN** there is a row in `llm_requests` with status = "error" and non-empty error_message