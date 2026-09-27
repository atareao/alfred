# Message: Location Field

## ADDED Requirements

### Requirement: Message model SHALL include optional location field

**Given** un mensaje en la base de datos  
**When** se recupera via API  
**Then** el campo `location` SHALL ser `Option<String>`  
**And** SHALL ser `null` cuando no hay ubicación configurada  
**And** SHALL contener la dirección textual (ej. "Silla, Valencia, España") cuando está disponible

#### Scenario: Location se persiste al crear mensaje con ubicación disponible
**Given** settings contienen `latitude`, `longitude` y `location_name`  
**When** `MessagesRepo::create()` es llamado  
**Then** el mensaje creado SHALL incluir `location` con el valor de `location_name` de settings

#### Scenario: Location es null cuando no hay ubicación
**Given** settings NO contienen `latitude` ni `location_name`  
**When** `MessagesRepo::create()` es llamado  
**Then** el mensaje creado SHALL tener `location = None`

#### Scenario: Migration añade columna location idempotentemente
**Given** una base de datos con tabla `messages` existente  
**When** `run_migrations()` se ejecuta  
**Then** la tabla SHALL tener columna `location TEXT` nullable  
**And** ejecutar migrations dos veces SHALL NO fallar

#### Scenario: LIST devuelve location
**Given** un mensaje con `location = "Madrid, España"`  
**When** `list_all()` es llamado  
**Then** el mensaje devuelto SHALL incluir `location = Some("Madrid, España")`

#### Scenario: find_by_id devuelve location
**Given** un mensaje con `location = "Barcelona"`  
**When** `find_by_id(msg_id)` es llamado  
**Then** el mensaje devuelto SHALL incluir `location = Some("Barcelona")`