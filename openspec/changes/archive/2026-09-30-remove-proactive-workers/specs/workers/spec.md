# Spec Delta: workers

## REMOVED Requirements

### Requirement: Briefing worker SHALL generate daily briefing on each tick

### Requirement: Conflict-detector worker SHALL check for scheduling conflicts

### Requirement: Travel-prep worker SHALL prepare trip suggestions

## MODIFIED Requirements

### Requirement: WorkerPool shutdown_tx SHALL be retained for application lifetime

**Given** a `WorkerPool` is created in `new_with_orchestrator()`  
**When** the pool finishes starting all workers  
**Then** the `shutdown_tx` broadcast sender SHALL NOT be dropped  
**And** all workers SHALL continue running until the application exits

#### Scenario: Workers survive beyond scope of new_with_orchestrator
**Given** `WorkerPool::start()` is called in `new_with_orchestrator`  
**When** the function returns the `AppState`  
**Then** all workers SHALL still be running (no "shutting down" log emitted)  
**And** `shutdown_tx` SHALL be retained in `AppState`

#### Scenario: Worker pool runs on startup
**Given** Valet is started  
**When** the server begins listening  
**Then** the Collapse worker SHALL still be running  
**And** the EpisodicMemoryWorker SHALL still be running
