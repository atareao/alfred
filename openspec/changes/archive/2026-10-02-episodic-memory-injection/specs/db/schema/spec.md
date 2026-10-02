# Spec Delta: db/schema

## ADDED Requirements

### Requirement: La migración SHALL crear la tabla virtual vec_memory con vec0

La migración SHALL crear `vec_memory` como tabla virtual `vec0` con la columna de id textual **declarada explícitamente** y una columna vectorial tipada con `distance_metric=cosine`: `CREATE VIRTUAL TABLE vec_memory USING vec0(id TEXT PRIMARY KEY, embedding float[N] distance_metric=cosine)`. Sin declarar `id`, la tabla solo tendría el `rowid` implícito y no se podría hacer el JOIN por id con `memory`. `memory` SHALL seguir siendo la fuente de verdad. La dimensión `N` SHALL coincidir con `EMBEDDING_DIMENSION`.

**Given** una base de datos migrada  
**When** se inspecciona `sqlite_master`  
**Then** `vec_memory` SHALL ser una tabla virtual `vec0`  
**And** SHALL exponer la columna `id` textual como clave primaria  
**And** SHALL declarar `embedding float[N]` con `distance_metric=cosine`  
**And** `memory` SHALL conservarse como fuente de verdad, con el JOIN por `id`

#### Scenario: Tabla virtual creada con columna id explícita
**Given** una base de datos vacía  
**When** se ejecuta `run_migrations()`  
**Then** `sqlite_master` contiene `vec_memory` de tipo `table` con SQL de `vec0`  
**And** la definición incluye `id TEXT PRIMARY KEY` y `embedding float[1024] distance_metric=cosine`

#### Scenario: El JOIN por id es posible
**Given** una fila en `memory` y su vector en `vec_memory` con el mismo `id`  
**When** se ejecuta `SELECT m.id, v.distance FROM memory m JOIN vec_memory v ON m.id = v.id WHERE v.embedding MATCH ? AND k = ? ORDER BY v.distance`  
**Then** la consulta devuelve la ficha con su distancia  
**And** NO falla con `table vec_items has no column named id`

#### Scenario: Migración idempotente
**Given** una base de datos ya migrada  
**When** se ejecuta `run_migrations()` de nuevo  
**Then** no se produce error  
**And** `vec_memory` sigue existiendo como tabla virtual

#### Scenario: La dimensión declarada coincide con EMBEDDING_DIMENSION
**Given** `EMBEDDING_DIMENSION = 1024`  
**When** se ejecuta `run_migrations()`  
**Then** `vec_memory` declara `embedding float[1024]`
