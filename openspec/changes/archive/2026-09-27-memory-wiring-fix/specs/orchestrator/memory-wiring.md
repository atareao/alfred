# orchestrator Specification — Memory wiring

## Requirements

### Requirement: Orchestrator SHALL have memory_tx field

**Given** el `Orchestrator` struct  
**When** se define  
**Then** SHALL tener `memory_tx: Option<mpsc::Sender<()>>`  
**And** SHALL initializarlo via `Orchestrator::new()`

#### Scenario: Orchestrator stores memory_tx
**Given** un `Orchestrator` creado con `memory_tx: Some(tx)`  
**When** se accede al campo  
**Then** `self.memory_tx` es `Some`

### Requirement: Orchestrator SHALL signal memory worker after persisting user messages

**Given** el agente persiste un mensaje de usuario en `process_message()`  
**When** `MessagesRepo::create()` retorna éxito (línea ~828)  
**Then** SHALL enviar `()` por `self.memory_tx` si es `Some`

#### Scenario: Signal sent on user message persist
**Given** `memory_tx` es `Some`  
**When** se persiste mensaje de usuario  
**Then** `memory_tx.send(()).await` se ejecuta

### Requirement: Orchestrator SHALL signal memory worker after persisting assistant messages

**Given** el agente persiste un mensaje de asistente en `process_message()`  
**When** `MessagesRepo::create()` retorna éxito (línea ~1107)  
**Then** SHALL enviar `()` por `self.memory_tx` si es `Some`

#### Scenario: Signal sent on assistant message persist
**Given** `memory_tx` es `Some`  
**When** se persiste mensaje de asistente  
**Then** `memory_tx.send(()).await` se ejecuta

### Requirement: AppState::new_with_orchestrator SHALL wire memory_tx from WorkerPool

**Given** `AppState::new_with_orchestrator()`  
**When** crea el `WorkerPool`  
**Then** SHALL exponer `WorkerPool.memory_tx` en `AppState.memory_tx`

#### Scenario: Production AppState has memory_tx
**Given** `AppState::new_with_orchestrator()`  
**When** se invoca con credenciales válidas  
**Then** `state.memory_tx` es `Some`

### Requirement: main SHALL wire memory_tx into AppState

**Given** `main.rs`  
**When** construye el estado de producción  
**Then** SHALL pasar `WorkerPool.memory_tx` a `AppState.memory_tx`

#### Scenario: main creates WorkerPool and wires memory_tx
**Given** `main()`  
**When** arranca  
**Then** crea un `WorkerPool::start()`  
**And** asigna `pool.memory_tx` a `AppState.memory_tx`