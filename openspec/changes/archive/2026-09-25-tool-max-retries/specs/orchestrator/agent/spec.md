## ADDED Requirements

### Requirement: Per-tool max retry limit of 3 in ReAct loop
**Given** el orquestador ejecuta el ReAct loop
**When** un tool es invocado 3 veces o más en el mismo loop
**Then** NO DEBE ejecutarse el tool otra vez
**And** SE DEBE enviar un mensaje al LLM indicando que el tool no está disponible tras 3 intentos
**And** el ReAct loop DEBE continuar para que el LLM responda con alternativas

#### Scenario: Tool fails 3 times, 4th call is blocked
**Given** un orquestador con un tool que siempre falla
**When** el LLM llama al tool por 4ª vez en el mismo ReAct loop
**Then** el tool NO se ejecuta
**And** el LLM recibe el mensaje "Tool 'X' has been called 3 times. No more retries allowed."
**And** el ReAct loop continúa

#### Scenario: Successful tool calls don't count towards limit
**Given** un orquestador
**When** un tool se ejecuta con éxito 2 veces
**Then** el contador del tool aumenta a 2
**And** aún se puede llamar una 3ª vez
**And** el límite de 3 aplica tanto a fallos como a éxitos

#### Scenario: Different tools have independent counters
**Given** un orquestador
**When** el tool "geo" se ha llamado 3 veces y el tool "weather" 1 vez
**Then** "geo" está bloqueado
**And** "weather" aún puede ejecutarse