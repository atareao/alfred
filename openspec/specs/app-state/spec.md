# AppState Specification

## Purpose
Define the shared application state struct that is passed to all route handlers via Axum's `State` extractor.

## Requirements

### Requirement: AppState SHALL retain WorkerPool shutdown_tx

**Given** `AppState` is the shared application state  
**When** it is constructed via `new_with_orchestrator()`  
**Then** it SHALL hold a `shutdown_tx: Option<broadcast::Sender<()>>` field  
**And** this field SHALL be `Some(...)` after construction  
**And** the value SHALL be the same sender used by `WorkerPool`

#### Scenario: shutdown_tx is retained in AppState
**Given** `new_with_orchestrator` is called  
**When** it creates the `WorkerPool`  
**Then** `shutdown_tx` SHALL be moved from the pool into `AppState`  
**And** `worker_pool.shutdown_tx` SHALL be moved out (partial move)

#### Scenario: new_in_memory variants have shutdown_tx = None
**Given** `AppState::new_in_memory()` or `AppState::new_in_memory_empty()`  
**When** construction completes  
**Then** `shutdown_tx` SHALL be `None`

#### Scenario: Clone does not duplicate the sender
**Given** an `AppState` with `shutdown_tx = Some(sender)`  
**When** it is cloned via `Clone` derive  
**Then** the clone SHALL also have `shutdown_tx = Some(sender)` (sender is cloned, broadcast allows)