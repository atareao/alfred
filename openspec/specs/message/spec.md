# message Specification

## Purpose
TBD - created by archiving change message-schema-enrichment. Update Purpose after archive.

## Requirements

### Requirement: estimate_tokens function SHALL compute token estimate
**Given** a text string  
**When** `estimate_tokens(text)` is called  
**Then** it SHALL return `((text.chars().count() as f64) / 3.5).ceil() as usize + 4`

#### Scenario: estimate_tokens with empty text
**Given** empty text ""  
**When** `estimate_tokens("")` is called  
**Then** it SHALL return 4

#### Scenario: estimate_tokens with short text
**Given** text "Hola"  
**When** `estimate_tokens("Hola")` is called  
**Then** it SHALL return 6

#### Scenario: estimate_tokens with long text
**Given** text of 3500 chars  
**When** `estimate_tokens(text)` is called  
**Then** it SHALL return 1004

### Requirement: Message model SHALL include new fields
**Given** a message stored in the database  
**Then** it SHALL have fields `tokens_count`, `collapsed_content`, `collapsed_tokens_count`, `is_indexed`, `summary_ref`

#### Scenario: Creation computes tokens_count automatically
**Given** content "Hola, ¿cómo estás?" (20 chars)  
**When** saved via `MessagesRepo::create`  
**Then** `msg.tokens_count` SHALL equal `estimate_tokens(content)`  
**And** `msg.collapsed_content` SHALL be `None`  
**And** `msg.collapsed_tokens_count` SHALL be `0`  
**And** `msg.is_indexed` SHALL be `false`

### Requirement: Collapse threshold SHALL be configurable
**Given** `COLLAPSE_THRESHOLD_TOKENS` env var  
**When** `Config::from_env()` is called  
**Then** `collapse_threshold_tokens` SHALL take the env var value or default to 2000

#### Scenario: Short message SHALL NOT trigger collapse
**Given** `collapse_threshold_tokens` = 2000  
**Given** a message with 100 chars  
**When** saved  
**Then** `tokens_count` SHALL be 33  
**And** the collapse callback SHALL NOT be invoked

#### Scenario: Long message SHALL trigger collapse
**Given** `collapse_threshold_tokens` = 2000  
**Given** a message with 8000 chars  
**When** saved  
**Then** `tokens_count` SHALL be >= 2000  
**And** the collapse callback SHALL be invoked with the message `id`

### Requirement: Migration SHALL add new columns idempotently
**Given** a database with an existing `messages` table  
**When** `run_migrations()` is executed  
**Then** the table SHALL have columns `tokens_count`, `collapsed_content`, `collapsed_tokens_count`, `is_indexed`, `summary_ref`  
**And** running migrations twice SHALL NOT fail

#### Scenario: New columns exist after migration
**Given** a fresh in-memory database  
**When** `run_migrations()` is executed  
**Then** `PRAGMA table_info(messages)` SHALL include `tokens_count`, `collapsed_content`, `collapsed_tokens_count`, `is_indexed`, `summary_ref`

#### Scenario: Migration is idempotent
**Given** a database where `run_migrations()` has already been executed  
**When** `run_migrations()` is executed again  
**Then** it SHALL succeed without error
