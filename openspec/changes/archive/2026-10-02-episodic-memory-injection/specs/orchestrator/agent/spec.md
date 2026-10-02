# Spec Delta: orchestrator/agent

## ADDED Requirements

### Requirement: Inyección automática del bloque de memoria episódica

**Given** un mensaje del usuario  
**When** el orquestador construye la petición al LLM  
**Then** SHALL recuperar memorias episódicas por similitud semántica en **cada** mensaje, con independencia de la estrategia de contexto  
**And** SHALL componer el bloque de memoria en código, no desde un placeholder de `settings.system_prompt`  
**And** SHALL omitir el bloque por completo cuando no haya ninguna ficha que supere el umbral, sin texto de relleno del tipo "no hay antecedentes"

#### Scenario: Se inyecta memoria cuando hay fichas sobre el umbral
**Given** un orquestador con pool, provider y al menos una ficha que supera `SIMILARITY_THRESHOLD`  
**When** se construye la petición  
**Then** el array de mensajes contiene el bloque de memoria episódica  
**And** el bloque incluye el contenido de esa ficha

#### Scenario: El bloque se omite por completo sin fichas
**Given** un orquestador con pool y provider pero sin fichas que superen el umbral  
**When** se construye la petición  
**Then** el array de mensajes NO contiene ningún mensaje de memoria episódica  
**And** NO aparece ningún texto de relleno del tipo "no hay antecedentes"

#### Scenario: Mismo comportamiento en los dos caminos
**Given** un orquestador con fichas que superan el umbral  
**When** se ejecuta `process_message()`  
**Then** la petición contiene el bloque de memoria episódica  
**And** `process_message_stream()` produce el mismo bloque que `process_message()`

#### Scenario: El bloque no depende de un placeholder editable
**Given** un `settings.system_prompt` sin ningún placeholder de memoria  
**When** se construye la petición  
**Then** el bloque de memoria se compone igualmente en código  
**And** borrar o editar `system_prompt` NO desactiva la memoria

### Requirement: Aislamiento estructural del bloque <episodic_memory>

El bloque de memoria episódica SHALL ir delimitado por las etiquetas `<episodic_memory>` dentro de la sección `# CONTEXTO DE MEMORIA EPISÓDICA (CAPA B)`, con la instrucción explícita de que son antecedentes y no parte de la conversación actual, y SHALL NOT usar el prefijo `[Memory context]`. El bloque SHALL ir **antes** del historial de conversación.

**Given** un orquestador con fichas que superan el umbral  
**When** se construye la petición  
**Then** el prompt resultante contiene `# CONTEXTO DE MEMORIA EPISÓDICA (CAPA B)` y las etiquetas `<episodic_memory>`  
**And** contiene la instrucción de que son antecedentes y no parte del turno actual  
**And** NO contiene el prefijo `[Memory context]`  
**And** el bloque aparece antes de los mensajes del historial de conversación

#### Scenario: Etiquetas e instrucción presentes
**Given** fichas inyectables  
**When** se inspecciona el mensaje de memoria del prompt  
**Then** contiene `<episodic_memory>` y su cierre  
**And** contiene la instrucción de no confundir los antecedentes con el turno actual

#### Scenario: El bloque precede al historial
**Given** un historial de conversación cargado por `list_by_token_budget`  
**When** se ordenan los mensajes de la petición  
**Then** el bloque `<episodic_memory>` aparece antes del primer mensaje del historial

#### Scenario: No queda el prefijo antiguo
**Given** fichas inyectables  
**When** se construye el prompt  
**Then** NO aparece el literal `[Memory context]` en ningún mensaje

### Requirement: Anclaje temporal y orden por decaimiento de cada ficha

Cada ficha inyectada SHALL precederse de su fecha derivada de `memory.created_at` para dar cronología al modelo. El orden de las fichas dentro del bloque SHALL ser el de relevancia final descendente definido en el requisito «El decaimiento temporal SHALL calcularse en Rust y ordenar los resultados» de `specs/orchestrator/spec.md`, de modo que la antigüedad reordena, pero no excluye.

**Given** una ficha con `created_at` no nulo  
**When** se compone el bloque de memoria episódica  
**Then** el texto inyectado de esa ficha SHALL incluir su fecha derivada de `created_at`  
**And** el orden de las fichas en el bloque SHALL seguir el de relevancia final descendente fijado en el requisito del decaimiento de `orchestrator`

#### Scenario: El texto inyectado incluye la fecha
**Given** una ficha con `created_at = "2026-09-29T10:00:00Z"`  
**When** se compone el bloque  
**Then** el texto de la ficha contiene la fecha derivada de ese `created_at`

#### Scenario: El orden es por relevancia final
**Given** dos fichas con la misma similitud y distinta antigüedad  
**When** se compone el bloque  
**Then** la ficha más reciente aparece antes que la más antigua

### Requirement: El formato de ficha SHALL NOT depender de metadata ausente

El formato de ficha SHALL NOT incluir el `[tags]`, porque el `EpisodicMemoryWorker` no escribe la clave `tags` en `metadata`.

**Given** una ficha persistida por el `EpisodicMemoryWorker` con `metadata = {"source","primary_message_ids","date_context"}` y sin `tags`  
**When** se formatea esa ficha para el bloque de memoria  
**Then** el texto SHALL NOT contener el prefijo `[tags]` ni corchetes vacíos (`[]`)

#### Scenario: Ficha del worker sin corchetes vacíos
**Given** una ficha con `metadata` sin la clave `tags`  
**When** se formatea la ficha  
**Then** el resultado NO contiene `[]`  
**And** el resultado NO contiene el prefijo `[tags]`
