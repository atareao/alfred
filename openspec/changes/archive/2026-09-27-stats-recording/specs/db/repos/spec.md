## ADDED Requirements

### Requirement: StatsRepo SHALL record LLM requests on each chat call
StatsRepo SHALL provide `record_request` to insert LLM usage data into the `llm_requests` table.

#### Scenario: inserts row with all fields
- **WHEN** `StatsRepo::record_request(pool, "req-1", "gpt-4o", "profile-1", 100, 50, 150, 0, 0, 0.0, Some(200), "success", None, None)` is called
- **THEN** the table contains 1 row with matching field values
- **AND** created_at is NOT NULL

#### Scenario: with error status
- **WHEN** `record_request` is called with status "error" and error_message Some("timeout")
- **THEN** the inserted row has status = "error"
- **AND** error_message = "timeout"

#### Scenario: auto-assigns created_at
- **WHEN** `record_request` is called with created_at = None
- **THEN** created_at is NOT NULL (database assigned datetime('now'))