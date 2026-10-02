# Spec Delta: embeddings

## MODIFIED Requirements

### Requirement: Reindex command SHALL regenerate all memory embeddings

`reindex_all(pool, provider, dimension)` SHALL recorrer todas las filas de `memory`, regenerar su embedding con el provider configurado y escribir la fila correspondiente en la tabla virtual `vec_memory` (`vec0`), reemplazando la fila por `id`. SHALL devolver un informe con el número de filas actualizadas y fallidas. SHALL NOT escribir el embedding como texto JSON: el vector SHALL almacenarse en el formato binario de `vec0`.

**Given** N filas en la tabla `memory` y un provider de embeddings configurado  
**When** se ejecuta `reindex_all(pool, provider, dimension)`  
**Then** SHALL regenerar el embedding de cada fila  
**And** SHALL escribir el vector binario en `vec_memory` reemplazando la fila por `id`  
**And** SHALL NOT escribir el embedding como texto JSON  
**And** SHALL devolver `ReindexReport { total, updated, failed }`

#### Scenario: Re-indexa todas las filas
**Given** 3 filas en `memory` y un provider mock  
**When** se ejecuta `reindex_all`  
**Then** `vec_memory` contiene 3 embeddings regenerados en formato binario de `vec0`  
**And** el informe indica `updated = 3`

#### Scenario: Idempotente
**Given** un re-indexado ya ejecutado  
**When** se ejecuta de nuevo  
**Then** `vec_memory` sigue conteniendo el mismo número de filas  
**And** los embeddings se reemplazan sin duplicar

#### Scenario: Fallo en una fila no aborta el resto
**Given** un provider que falla para una fila concreta  
**When** se ejecuta `reindex_all`  
**Then** las demás filas se re-indexan  
**And** el informe indica `failed = 1`

#### Scenario: Dimensión incorrecta no deja vec_memory a medias
**Given** `EMBEDDING_DIMENSION = 1024` y un provider que devuelve vectores de 768 dims  
**When** se ejecuta `reindex_all`  
**Then** la operación falla con un error de dimensión  
**And** `vec_memory` no queda modificada
