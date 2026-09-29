# dependencies Specification

## Purpose

Define las reglas de compatibilidad y seguridad que el código debe respetar al usar las
dependencias Rust del proyecto (axum, sqlx, reqwest, tower-http, thiserror, base64, etc.),
especialmente tras actualizaciones de versión mayor que introducen breaking changes.

Cubre dos invariantes críticos:

- **SQL dinámico**: con sqlx 0.9, cualquier query construida con un `String` dinámico debe
auditarse y envolverse explícitamente con `sqlx::AssertSqlSafe(...)`; los valores de usuario
van siempre por bind parameters.
- **Rutas axum**: con axum 0.8, los segmentos de captura usan la sintaxis `{param}`.

Además, fija el criterio de aceptación de una actualización de dependencias: el proyecto
de compilar sin errores, pasar el 100% de los tests y no emitir warnings de clippy.

## Requirements

### Requirement: SQL dinámico SHALL auditarse explícitamente con sqlx 0.9

**Given** el crate `sqlx` en versión 0.9  
**When** se construye una query con un `String` dinámico (no `&'static str`)  
**Then** SHALL envolverse con `sqlx::AssertSqlSafe(...)` antes de pasarla a `sqlx::query()` o `sqlx::query_scalar()`  
**And** el SQL dinámico SHALL construirse solo a partir de fragmentos literales controlados por el código, nunca de input de usuario sin sanitizar

#### Scenario: Query dinámica con bind parameters
**Given** un `String` `sql` construido con fragmentos literales y placeholders `?1`  
**When** se ejecuta `sqlx::query(sqlx::AssertSqlSafe(sql)).bind(profile_id)`  
**Then** la query SHALL compilar y ejecutarse sin error

#### Scenario: Query dinámica sin binds
**Given** un `String` `query_str` construido con fragmentos literales  
**When** se ejecuta `sqlx::query(sqlx::AssertSqlSafe(query_str))`  
**Then** la query SHALL compilar y ejecutarse sin error

### Requirement: Rutas axum SHALL usar la sintaxis de captura `{param}`

**Given** el crate `axum` en versión 0.8  
**When** se registra una ruta con un segmento de captura  
**Then** SHALL usarse la sintaxis `{param}` (p. ej. `/api/messages/{msg_id}`)  
**And** NO SHALL usarse la sintaxis obsoleta `:param`

#### Scenario: Router construido sin panic
**Given** el router de la aplicación con todas sus rutas registradas  
**When** se construye el router  
**Then** NO SHALL producirse panic por sintaxis de ruta inválida

#### Scenario: Captura de path param en runtime
**Given** una petición `GET /api/messages/{msg_id}` con un id válido  
**When** se enruta la petición  
**Then** el handler SHALL recibir el valor del path param

### Requirement: El proyecto SHALL compilar y pasar la suite de tests con las versiones actualizadas

**Given** las dependencias en sus versiones mayores actualizadas  
**When** se ejecuta `cargo check --all-targets`  
**Then** SHALL compilar sin errores  
**And** `cargo test` SHALL pasar el 100% de los tests  
**And** `cargo clippy -- -D warnings` SHALL no emitir warnings

#### Scenario: Compilación limpia
**Given** el árbol de código adaptado a las nuevas versiones  
**When** se ejecuta `cargo check --all-targets`  
**Then** SHALL finalizar con código de salida 0 y sin errores

#### Scenario: Suite de tests en verde
**Given** el árbol de código adaptado a las nuevas versiones  
**When** se ejecuta `cargo test`  
**Then** SHALL pasar el 100% de los tests (510 lib + integración)  
**And** NO SHALL haber tests fallidos

#### Scenario: Linter sin warnings
**Given** el árbol de código adaptado a las nuevas versiones  
**When** se ejecuta `cargo clippy -- -D warnings`  
**Then** SHALL finalizar sin warnings
