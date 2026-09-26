## ADDED Requirements

### Requirement: CalendarView header responsivo

#### Scenario: Header se apila verticalmente en móvil
**Given** el viewport es < 768px  
**When** se abre el modal de agenda  
**Then** el selector de categoría y botón "New Event" están en columna (stack vertical)  
**And** ocupan el ancho completo disponible

#### Scenario: Header en fila en desktop
**Given** el viewport es ≥ 768px  
**When** se abre el modal de agenda  
**Then** el selector y botón están en fila horizontal (como ahora)

### Requirement: Modales con ancho dinámico

#### Scenario: EventModal se adapta en móvil
**Given** el viewport es < 768px  
**When** se abre EventModal  
**Then** el modal ocupa `'100vw'` menos padding (16px a cada lado)

#### Scenario: EventDetail se adapta en móvil
**Given** el viewport es < 768px  
**When** se abre EventDetail  
**Then** el modal ocupa `'100vw'` menos padding  
**And** las descripciones se muestran sin borde (mejor legibilidad)

### Requirement: Lista de eventos del día responsiva

#### Scenario: Eventos del día se apilan en móvil
**Given** el viewport es < 768px  
**When** se selecciona una fecha con eventos  
**Then** cada evento ocupa el ancho completo  
**And** título y hora están en vertical en vez de horizontal