# Spec Delta: db/repos

## ADDED Requirements

### Requirement: search_by_vector SHALL warn on embedding dimension mismatch

`MemoryRepo::search_by_vector` SHALL descartar los embeddings almacenados cuya longitud difiera de la del embedding de consulta y SHALL loguear un warning indicando ambas dimensiones, en lugar de fallar en silencio.

**Given** un embedding de consulta de dimensión D  
**When** `search_by_vector` encuentra un embedding almacenado de dimensión distinta de D  
**Then** SHALL descartarlo  
**And** SHALL loguear un warning con la dimensión de la consulta y la del embedding descartado  
**And** SHALL NOT devolver `0.0` en silencio

#### Scenario: Dimensión incompatible se descarta con warning
**Given** una consulta de dimensión 1536 y un embedding almacenado de dimensión 384  
**When** se ejecuta `search_by_vector`  
**Then** el embedding de 384 se descarta  
**And** se loguea un warning con ambas dimensiones

#### Scenario: Dimensión compatible se puntúa con normalidad
**Given** una consulta de dimensión 3 y un embedding almacenado de dimensión 3  
**When** se ejecuta `search_by_vector`  
**Then** el embedding se puntúa por similitud coseno  
**And** no se loguea ningún warning de dimensión
