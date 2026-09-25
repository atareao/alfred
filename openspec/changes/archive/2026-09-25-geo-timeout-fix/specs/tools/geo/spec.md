## ADDED Requirements

### Requirement: Configurable Overpass base URL
**Given** un `GeoTool`
**When** se crea con `GeoTool::new(db)`
**Then** la URL base de Overpass DEBE ser `https://overpass-api.de/api/interpreter`
**And** SE DEBE poder crear con `GeoTool::with_base_url(db, otra_url)` para tests

#### Scenario: Default URL points to production Overpass
**Given** un pool de base de datos
**When** se crea `GeoTool::new(pool)`
**Then** el campo `overpass_base_url` es `"https://overpass-api.de/api/interpreter"`

#### Scenario: Custom URL for testing
**Given** un pool de base de datos y una URL custom
**When** se crea `GeoTool::with_base_url(pool, "http://localhost:9999/api")`
**Then** las requests de `search_places` se envían a esa URL

### Requirement: HTTP timeout of 30 seconds
**Given** un `GeoTool`
**When** se crea con `GeoTool::new(db)` o `GeoTool::with_base_url(db, url)`
**Then** el `reqwest::Client` interno DEBE tener `timeout(Duration::from_secs(30))`

#### Scenario: Timeout configurado en constructor
**Given** un pool de base de datos
**When** se crea `GeoTool::new(pool)`
**Then** el comando `new()` no paniquea y el cliente se construye con timeout

#### Scenario: Slow server causes timeout error
**Given** un `GeoTool` con un mock HTTP server que tarda >30s en responder
**When** se ejecuta `search_places`
**Then** retorna `Err(ToolError::ExecutionError)` con mensaje conteniendo "timeout"

### Requirement: Raw body on error responses
**Given** `search_places` ejecuta una request HTTP a Overpass
**When** la respuesta tiene status code que no es success (e.g. 429, 502)
**Then** se lee el body como texto raw con `resp.text().await`
**And** se retorna `ToolError::ExecutionError` con status code y raw body

#### Scenario: Overpass returns 429 Too Many Requests
**Given** un `GeoTool` con un mock HTTP client
**When** `search_places` se ejecuta
**And** Overpass responde con status 429 y body HTML `<html><body>Rate limited</body></html>`
**Then** retorna `Err(ToolError::ExecutionError("Overpass returned status 429: <html><body>Rate limited</body></html>"))`

#### Scenario: Overpass returns 502 Bad Gateway
**Given** un `GeoTool` con un mock HTTP client
**When** `search_places` se ejecuta
**And** Overpass responde con status 502 y body `{"error":"upstream"}`
**Then** retorna `Err(ToolError::ExecutionError("Overpass returned status 502: {\"error\":\"upstream\"}"))`

### Requirement: Full raw body on success (no field filtering)
**Given** `search_places` ejecuta una request HTTP a Overpass
**When** la respuesta tiene status 200 y body JSON válido
**Then** se devuelve el `Value` completo del body de Overpass sin filtrar como `ToolResult.data`
**And** NO se extraen campos individuales (name, category, distance_m)
**And** NO se calcula distancia haversine

#### Scenario: Success returns full Overpass JSON
**Given** un `GeoTool` con un mock HTTP client
**When** `search_places` se ejecuta
**And** Overpass responde con 200 y JSON `{"elements":[{"type":"node","tags":{"name":"Café Central"}}]}`
**Then** `ToolResult.data` contiene el Value completo: `{"elements":[{"type":"node","tags":{"name":"Café Central"}}]}`
**And** NO se filtran campos individuales
**And** NO se calcula distancia haversine

## ADDED Requirements

### Requirement: Escaping for Overpass QL queries
**Given** la función `overpass_escape`
**When** se llama con una string que contiene `\` o `"`
**Then** retorna la string con esos caracteres escapados
**And** los caracteres especiales de regex (`.`, `*`, `+`, `(`, `)`) NO se escapan

#### Scenario: Backslash y double quote se escapan
**Given** la función `overpass_escape`
**When** se llama con `"\\\""` (backslash seguido de comilla doble)
**Then** retorna `"\\\\\\\""` (ambos escapados)

#### Scenario: Regex chars no se escapan
**Given** la función `overpass_escape`
**When** se llama con `"cafe (bar)"`
**Then** retorna `"cafe (bar)"` (sin cambios)