# Spec Delta: frontend

## ADDED Requirements

### Requirement: SettingsDialog SHALL display a Memoria tab with the four memory knobs

SettingsDialog SHALL display a "Memoria" tab with four numeric fields, one per memory knob: `MEMORY_HALF_LIFE_DAYS`, `SIMILARITY_THRESHOLD`, `RAG_BUDGET_TOKENS` and `MEMORY_KNN_CANDIDATES`. Los valores SHALL cargarse de `GET /settings` y guardarse con `PUT /settings`, sin rutas nuevas de API. Al ser ajustables en caliente, un cambio guardado SHALL surtir efecto sin reiniciar.

**Given** el SettingsDialog está abierto en la tab "Memoria"  
**When** se renderiza  
**Then** muestra cuatro campos numéricos: `MEMORY_HALF_LIFE_DAYS`, `SIMILARITY_THRESHOLD`, `RAG_BUDGET_TOKENS` y `MEMORY_KNN_CANDIDATES`  
**And** cada campo muestra el valor actual cargado de `GET /settings`  
**And** un botón "Guardar" persiste los cuatro valores vía `PUT /settings`

#### Scenario: Los cuatro campos están presentes
**Given** el SettingsDialog está abierto en la tab "Memoria"  
**When** se renderiza  
**Then** existen los campos `MEMORY_HALF_LIFE_DAYS`, `SIMILARITY_THRESHOLD`, `RAG_BUDGET_TOKENS` y `MEMORY_KNN_CANDIDATES`

#### Scenario: Los valores se cargan desde la BD
**Given** `GET /settings` devuelve `MEMORY_HALF_LIFE_DAYS = 90` y `RAG_BUDGET_TOKENS = 800`  
**When** se abre la tab "Memoria"  
**Then** el campo `MEMORY_HALF_LIFE_DAYS` muestra `90`  
**And** el campo `RAG_BUDGET_TOKENS` muestra `800`

#### Scenario: Los cuatro mandos se guardan
**Given** el usuario edita los cuatro campos  
**When** hace clic en "Guardar"  
**Then** `updateSettings` se llama con `{ MEMORY_HALF_LIFE_DAYS, SIMILARITY_THRESHOLD, RAG_BUDGET_TOKENS, MEMORY_KNN_CANDIDATES }`  
**And** se muestra el mensaje "Ajustes guardados"

#### Scenario: Editar un mando no borra los otros
**Given** el usuario modifica solo `SIMILARITY_THRESHOLD`  
**When** hace clic en "Guardar"  
**Then** los otros tres mandos se envían con sus valores actuales sin cambios
