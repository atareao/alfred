# workers Specification — EpisodicMemoryWorker

## Requirements

### Requirement: Worker SHALL be triggered by two sources

**Given** un `EpisodicMemoryWorker` que escucha en un canal `mpsc::Receiver`  
**And** un timer periódico de `MEMORY_POLL_INTERVAL` (default: 30 minutos)  
**When** llega un signal por el canal (mensaje insertado) O el timer se cumple  
**Then** el worker SHALL ejecutar su ciclo de evaluación

#### Scenario: Worker loop con select! de dos fuentes
**Given** un worker iniciado  
**When** se inspecciona el loop principal  
**Then** usa `tokio::select!` con dos brazos: `chan.recv()` y `interval.tick()`

#### Scenario: Timer trigger sin mensajes nuevos
**Given** mensajes sin indexar desde hace > 30 min  
**When** el timer de 30 min se cumple  
**Then** el worker ejecuta el ciclo  
**And** procesa el batch

#### Scenario: No unindexed messages on timer
**Given** todos los mensajes tienen `is_indexed = 1`  
**When** el timer se cumple  
**Then** el worker ejecuta el ciclo  
**And** no hace nada (0 mensajes sin indexar)

### Requirement: Worker SHALL query unindexed messages on trigger

**Given** el worker recibe un trigger (signal o timer)  
**When** ejecuta su ciclo  
**Then** SHALL seleccionar mensajes con `is_indexed = 0` ordenados por `created_at ASC`  
**And** SHALL sumar `tokens_count` hasta alcanzar `MEMORY_BATCH_TOKENS` (default: 2000)

#### Scenario: Selects unindexed messages up to token budget
**Given** 10 mensajes sin indexar, cada uno de 300 tokens  
**When** el worker ejecuta el ciclo  
**Then** toma 7 mensajes (2100 tokens >= 2000)  
**And** procesa el batch

#### Scenario: No unindexed messages
**Given** todos los mensajes tienen `is_indexed = 1`  
**When** el worker ejecuta su ciclo  
**Then** no hace nada (no envía al LLM)

### Requirement: Worker SHALL detect inactivity timeout

**Given** mensajes sin indexar cuya suma de tokens NO alcanza `MEMORY_BATCH_TOKENS`  
**When** el tiempo desde el `created_at` del mensaje sin indexar más antiguo supera `MEMORY_INACTIVITY_MINUTES` (default: 30)  
**Then** SHALL procesar el batch aunque no se alcance el tope de tokens

#### Scenario: Inactivity triggers early batch
**Given** 3 mensajes sin indexar (500 tokens totales, < 2000)  
**When** el mensaje más antiguo del batch tiene created_at > 30 min respecto a now  
**Then** el worker procesa el batch con esos 3 mensajes

#### Scenario: Not enough messages and not enough time
**Given** 3 mensajes sin indexar (500 tokens, < 2000)  
**When** el mensaje más antiguo tiene solo 5 minutos  
**Then** el worker NO procesa el batch  
**And** espera al próximo trigger

### Requirement: Worker SHALL include overlap messages

**Given** un batch de N mensajes principales sin indexar  
**When** se prepara el bloque para el LLM  
**Then** SHALL incluir `MEMORY_OVERLAP` mensajes ANTERIORES al batch (default: 2, ya indexados)  
**And** SHALL incluir `MEMORY_OVERLAP` mensajes POSTERIORES al batch (default: 2, podrían estar o no indexados)

#### Scenario: Overlap at boundaries
**Given** messages batch desde msg-5 hasta msg-10  
**When** se construye el bloque  
**Then** incluye msg-3, msg-4 (anteriores) y msg-11, msg-12 (posteriores)  
**And** marca como "overlap" en el prompt

#### Scenario: No messages before batch
**Given** el batch empieza en el primer mensaje  
**When** se construye el bloque  
**Then** solo incluye overlap posterior (2 mensajes después)

### Requirement: Worker SHALL call LLM with archivist prompt

**Given** un bloque de mensajes preparado  
**When** se envía al LLM  
**Then** SHALL usar el prompt de archivista con formato estructurado  
**And** SHALL extraer: FECHA/CONTEXTO, TEMAS TRATADOS, HECHOS Y DECISIONES, SÍNTESIS  
**And** SHALL usar `MEMORY_MODEL` de config (default: collapse_model)

#### Scenario: LLM generates episodic card
**Given** bloque de 5 mensajes sobre configuración de Podman  
**When** worker llama al LLM  
**Then** recibe una ficha con FECHA/CONTEXTO, TEMAS TRATADOS (contiene "Podman"), HECHOS Y DECISIONES, SÍNTESIS

### Requirement: Worker SHALL store memory + embedding + update messages

**Given** una ficha generada por el LLM  
**When** se persiste  
**Then** SHALL insertar en `memory` con `tokens_count = estimate_tokens(ficha)`  
**And** SHALL generar embedding de la ficha  
**And** SHALL insertar en `vec_memory(embedding)`  
**And** SHALL hacer `UPDATE messages SET is_indexed = 1, summary_ref = ? WHERE id IN (mensajes principales, sin overlap)`

#### Scenario: Persistencia completa del ciclo
**Given** batch de 7 mensajes principales + 4 overlap  
**When** worker completa el ciclo  
**Then** existe 1 fila en `memory`  
**And** existe 1 fila en `vec_memory`  
**And** los 7 mensajes principales tienen `is_indexed = 1` y `summary_ref` no nulo  
**And** los 4 mensajes de overlap NO tienen `is_indexed` modificado

### Requirement: Handler SHALL wire episodic signal on message create

**Given** un handler `create_message` con acceso al canal `memory_tx`  
**When** se crea un mensaje  
**Then** SHALL enviar un signal por el canal  
**And** SHALL hacerlo en un `tokio::spawn` para no bloquear la respuesta

#### Scenario: Signal sent on message create
**Given** `memory_tx` es un `mpsc::Sender`  
**When** `create_message` se ejecuta con éxito  
**Then** el sender envía `()` por el canal  
**And** el handler responde 201 sin esperar al worker

### Requirement: Agent SHALL wire episodic signal

**Given** el orquestador con acceso a `memory_tx`  
**When** persiste mensajes de usuario y asistente  
**Then** SHALL enviar un signal por el canal tras cada persistencia

#### Scenario: Agent sends signal
**Given** `Agent` tiene `memory_tx: Option<mpsc::Sender<()>>`  
**When** el agente guarda un mensaje en DB  
**Then** `memory_tx.send(()).await` se ejecuta (si `Some`)