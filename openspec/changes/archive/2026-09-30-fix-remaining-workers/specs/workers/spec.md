# Spec Delta: workers

## MODIFIED Requirements

### Requirement: Handler SHALL wire collapse callback on message creation

**Given** a message with `tokens_count >= collapse_threshold_tokens`  
**When** `create_message` handler is called  
**Then** it SHALL send the message ID through the collapse channel  
**And** the threshold SHALL be read from `config.collapse_threshold_tokens` (not hardcoded)

#### Scenario: Short message does not trigger collapse
**Given** a message with 100 chars  
**When** created via the API  
**Then** the collapse channel SHALL NOT receive the message ID

#### Scenario: Long message triggers collapse
**Given** a message with 8000 chars  
**When** created via the API  
**Then** the collapse channel SHALL receive the message ID

#### Scenario: Custom threshold is honoured
**Given** `COLLAPSE_THRESHOLD_TOKENS` = `500`  
**When** a message with ~600 tokens is created  
**Then** the collapse channel SHALL receive the message ID

## ADDED Requirements

### Requirement: Collapse channel SHALL NOT silently drop message IDs

**Given** the collapse channel is full  
**When** a message ID is sent  
**Then** the sender SHALL apply backpressure (await capacity) or log a warning  
**And** SHALL NOT discard the ID without any log

#### Scenario: Full channel does not lose the ID silently
**Given** a collapse channel with capacity 1 already holding one ID  
**When** a second ID is sent  
**Then** the ID SHALL be delivered once capacity frees up (or a warning SHALL be logged)

### Requirement: EpisodicMemoryWorker SHALL NOT re-call the LLM after a persist failure within the cooldown

**Given** the worker obtained a valid memory card from the LLM  
**When** `persist()` fails  
**Then** the worker SHALL start the cooldown window  
**And** SHALL NOT call the LLM again until the cooldown elapses

#### Scenario: Persist failure does not trigger an immediate LLM retry
**Given** a batch that produces a valid card but whose persist fails  
**When** `evaluate()` is called again within the cooldown  
**Then** the LLM SHALL NOT be called again

### Requirement: EpisodicMemoryWorker SHALL persist memory and embedding atomically

**Given** a memory card to persist  
**When** the worker writes to `memory` and `vec_memory`  
**Then** both writes SHALL happen in a single transaction  
**And** a failure in either SHALL leave no orphan row

#### Scenario: vec_memory failure leaves no orphan memory row
**Given** the `vec_memory` insert fails  
**When** `persist()` runs  
**Then** the `memory` table SHALL NOT contain a row for that card

### Requirement: Workers SHALL record LLM stats with a NULL profile_id

**Given** a worker (Collapse or EpisodicMemory) records an LLM request  
**When** it calls `StatsRepo::record_request`  
**Then** the `profile_id` SHALL be `NULL` (system operation)  
**And** SHALL NOT use a literal such as `"background"` or `"episodic"` that violates the FK

#### Scenario: Collapse stats are recorded with NULL profile
**Given** a database with one profile  
**When** the CollapseWorker records an LLM request  
**Then** the `llm_requests` row SHALL be inserted successfully  
**And** its `profile_id` SHALL be `NULL`

#### Scenario: Episodic stats are recorded with NULL profile
**Given** a database with one profile  
**When** the EpisodicMemoryWorker records an LLM request  
**Then** the `llm_requests` row SHALL be inserted successfully  
**And** its `profile_id` SHALL be `NULL`
