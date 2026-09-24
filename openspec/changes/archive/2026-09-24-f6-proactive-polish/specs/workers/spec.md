# Workers Proactivos

## Contracts

```rust
// ── WorkerPool ──────────────────────────────────────────────────────────

pub struct WorkerPool {
    pub briefing: Option<JoinHandle<()>>,
    pub conflict_detector: Option<JoinHandle<()>>,
    pub travel_prep: Option<JoinHandle<()>>,
    pub memory_consolidator: Option<JoinHandle<()>>,
}

impl WorkerPool {
    pub fn start(db: DbPool, config: &Config) -> Self;
    pub fn shutdown(self);
}

// ── Briefing Matutino ────────────────────────────────────────────────────

pub struct MorningBriefing {
    pub date: String,
    pub events: Vec<Event>,
    pub tasks: Vec<Task>,
    pub weather: Option<WeatherResponse>,
    pub reminders: Vec<Reminder>,
}

// BriefingWorker: cada día a las 08:15 (configurable) genera un resumen
// y lo envía como mensaje del sistema en la conversación principal.

// ── Detección de Conflictos ──────────────────────────────────────────────

pub struct ConflictAlert {
    pub event_a: Event,
    pub event_b: Event,
    pub gap_minutes: i64,
    pub travel_time_minutes: i64,
    pub severity: ConflictSeverity, // Warning | Critical
}

// ConflictDetector: cada vez que se crea/modifica un evento, verifica
// si hay solapamiento o tiempo insuficiente entre citas consecutivas.

// ── Preparación de Viajes ────────────────────────────────────────────────

pub struct TravelPrep {
    pub event_id: String,
    pub destination: String,
    pub weather_forecast: Option<Value>,
    pub suggestions: Vec<String>,
}

// TravelPrepWorker: N días antes de un viaje, compila clima, sugiere
// restaurantes según perfil, propone itinerario.

// ── Consolidación Nocturna ───────────────────────────────────────────────

use std::sync::{Arc, Mutex};

pub struct MemoryConsolidator {
    db: Arc<Mutex<Connection>>,
}

impl MemoryConsolidator {
    /// Create a new MemoryConsolidator with the shared DB connection.
    pub fn new(db: Arc<Mutex<Connection>>) -> Self;

    /// Run nightly consolidation. Returns a human-readable report of what was
    /// cleaned up (orphan embeddings, old conversations, etc.), or
    /// "✅ Todo en orden — no hay datos que limpiar" if nothing was done.
    pub fn consolidate(&self) -> Result<String, String>;
}
```

## Scenarios

### WorkerPool: start all workers
**Given** a valid Config with worker intervals  
**When** `WorkerPool::start()` is called  
**Then** 4 workers are spawned as tokio tasks

### WorkerPool: shutdown gracefully
**Given** a running WorkerPool  
**When** `shutdown()` is called  
**Then** all workers are aborted and the function returns

### Briefing: generates daily summary
**Given** events, tasks, and reminders exist for today  
**When** the briefing worker runs  
**Then** it creates a system message in the main conversation with the summary

### ConflictDetector: detects overlapping events
**Given** two events on the same day with <30min gap  
**When** the conflict detector runs  
**Then** it creates a ConflictAlert with severity Warning

### TravelPrep: prepares before trip
**Given** an event with location exists N days from now  
**When** the travel prep worker runs  
**Then** it fetches weather and generates suggestions

### MemoryConsolidator: empty database
**Given** an empty database with all tables created  
**When** `consolidate()` is called  
**Then** it returns Ok with "✅ Todo en orden — no hay datos que limpiar"

### MemoryConsolidator: cleans orphan message embeddings
**Given** a `message_embeddings` row whose id has no matching row in `messages`  
**When** `consolidate()` is called  
**Then** that orphan embedding is deleted  
**And** the report includes "Eliminados ... embeddings huérfanos de mensajes"

### MemoryConsolidator: cleans orphan memory embeddings
**Given** a `memory_embeddings` row whose id has no matching row in `memories`  
**When** `consolidate()` is called  
**Then** that orphan embedding is deleted  
**And** the report includes "Eliminados ... embeddings huérfanos de memorias"

### MemoryConsolidator: archives old conversations
**Given** a conversation with no messages in the last 30 days  
**When** `consolidate()` is called  
**Then** that conversation is deleted  
**And** the report includes "Archivadas ... conversaciones antiguas"

### MemoryConsolidator: preserves recent conversations
**Given** a conversation with a message from today  
**When** `consolidate()` is called  
**Then** the conversation is NOT deleted