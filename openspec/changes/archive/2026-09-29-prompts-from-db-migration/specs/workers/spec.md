# Spec Delta: workers

## MODIFIED Requirements

### Requirement: Collapse prompt SHALL be configurable via settings
**Given** the `settings` table has a `collapse_prompt` key  
**When** the worker starts  
**Then** it SHALL read `collapse_prompt` from settings  
**And** SHALL use it as the system prompt for the LLM call  
**And** SHALL fall back to a minimal default prompt if the setting is missing, empty, or cannot be read due to a database error

#### Scenario: Default collapse prompt
**Given** a freshly migrated database with the seeded `collapse_prompt`  
**When** the worker processes a message  
**Then** it SHALL use the seeded prompt containing "Resume el siguiente texto"

#### Scenario: Custom collapse prompt
**Given** `collapse_prompt` = "Summarize in 3 bullet points" in settings  
**When** the worker processes a message  
**Then** it SHALL use "Summarize in 3 bullet points" as the system prompt

#### Scenario: Collapse prompt read failure is distinguished in logs
**Given** reading `collapse_prompt` fails with a database error
**When** the worker starts
**Then** it SHALL log a warning including the error and use the minimal fallback

#### Scenario: Missing or empty collapse prompt is distinguished in logs
**Given** `collapse_prompt` is absent or empty in settings
**When** the worker starts
**Then** it SHALL log a warning stating the key is missing or empty and use the minimal fallback

## ADDED Requirements

### Requirement: Archivist prompt SHALL be loaded from settings

**Given** the `settings` table has an `archivist_prompt` key seeded by migration
**When** `EpisodicMemoryWorker::call_llm()` builds the LLM request
**Then** it SHALL read `archivist_prompt` from settings
**And** SHALL substitute the `{{ BLOQUE_DE_MENSAJES }}` placeholder with the message block
**And** SHALL NOT use a hardcoded constant
**And** SHALL fall back to a minimal default prompt if the setting is missing, empty, or cannot be read due to a database error

#### Scenario: Seeded archivist prompt is used
- **GIVEN** a freshly migrated database with the seeded `archivist_prompt`
- **WHEN** the worker calls the LLM
- **THEN** the system message contains "archivista de memoria"
- **AND** the `{{ BLOQUE_DE_MENSAJES }}` placeholder is replaced by the message block

#### Scenario: Custom archivist prompt is used
- **GIVEN** `archivist_prompt = "CUSTOM ARCHIVIST {{ BLOQUE_DE_MENSAJES }}"` in settings
- **WHEN** the worker calls the LLM
- **THEN** the system message starts with "CUSTOM ARCHIVIST "
- **AND** contains the message block

#### Scenario: Fallback when archivist prompt is missing
- **GIVEN** a database without `archivist_prompt`
- **WHEN** the worker calls the LLM
- **THEN** a minimal non-empty fallback prompt is used
- **AND** a warning stating the key is missing or empty is logged

#### Scenario: Fallback when archivist prompt read fails
- **GIVEN** reading `archivist_prompt` fails with a database error
- **WHEN** the worker calls the LLM
- **THEN** a minimal non-empty fallback prompt is used
- **AND** a warning including the error is logged