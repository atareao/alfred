# Spec Delta: dependencies

## ADDED Requirements

### Requirement: El proyecto SHALL registrar la extensión sqlite-vec sobre la instancia de libsqlite3-sys que usa sqlx

El proyecto SHALL declarar `sqlite-vec` con el feature `vec0` activado y `libsqlite3-sys = "0.37"` como **dependencia directa**, de modo que se registre la extensión sobre la misma instancia de `libsqlite3-sys` que usa `sqlx-sqlite` (Cargo unifica una única instancia). Al arrancar SHALL llamarse a `sqlite3_auto_extension(sqlite3_vec_init)` antes de abrir el pool, lo que cubre cada conexión al abrirse. `sqlite-vec 0.1.9` SHALL NOT declararse como dependiente de `libsqlite3-sys`.

**Given** el árbol de dependencias del proyecto  
**When** se resuelve `libsqlite3-sys`  
**Then** SHALL existir una **única** instancia compartida por el código del proyecto y `sqlx-sqlite`  
**And** `sqlite-vec` SHALL estar con el feature `vec0` activado  
**And** `libsqlite3-sys` SHALL ser dependencia directa

#### Scenario: Una única instancia de libsqlite3-sys
**Given** `libsqlite3-sys = "0.37"` como dependencia directa  
**When** se ejecuta `cargo tree -i libsqlite3-sys`  
**Then** aparece una única instancia `0.37.0`  
**And** es la misma que usa `sqlx-sqlite`

#### Scenario: La extensión queda registrada en cada conexión
**Given** la aplicación arrancada con `sqlite3_auto_extension(sqlite3_vec_init)`  
**When** se ejecuta `SELECT vec_version()` desde una conexión del pool y desde una conexión nueva  
**Then** ambas devuelven `v0.1.9`

### Requirement: El arranque SHALL fallar de forma explícita si la extensión sqlite-vec no carga

Si la extensión no está disponible, el sistema SHALL **no arrancar**, con un error explícito. Un asistente que arranca aparentemente bien pero sin memoria es un fallo silencioso inaceptable. La comprobación SHALL hacerse ejecutando `SELECT vec_version()` al arrancar.

**Given** el arranque de la aplicación  
**When** se ejecuta `SELECT vec_version()` como comprobación  
**Then** si la extensión responde, el arranque SHALL continuar  
**And** si la extensión no carga, el arranque SHALL fallar con un error explícito que indique qué falta (extensión no registrada / `vec0` no disponible)

#### Scenario: La extensión disponible permite arrancar
**Given** la extensión `sqlite-vec` registrada  
**When** se ejecuta `SELECT vec_version()` al arrancar  
**Then** devuelve la versión y el arranque continúa

#### Scenario: La extensión ausente aborta el arranque
**Given** una conexión sin la extensión `sqlite-vec` registrada  
**When** se ejecuta la comprobación de arranque  
**Then** el arranque falla  
**And** el error indica que la extensión no está registrada o que `vec0` no está disponible
