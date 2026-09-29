# geo/weather tools spec

## Purpose

Especificación de las tools de geolocalización y meteorología: la tool de tiempo basada en latitud/longitud y las tools `geocode`, `reverse_geocode` y `search_places` separadas de la tool geo original.

## Requirements

### Requirement: Weather tool — remove operation, add required lat/lon

La tool de tiempo SHALL requerir `latitude` y `longitude` y SHALL NOT exponer un parámetro `operation`.

#### Scenario: Weather tool has no operation parameter
**Given** the weather tool definition
**When** `parameters()` is called
**Then** the schema must NOT have an `operation` property
**And** `required` must contain `["latitude", "longitude"]`

#### Scenario: Weather tool called with lat/lon returns data
**Given** a weather tool with a valid API key
**When** `execute({"latitude": 40.41, "longitude": -3.70})` is called
**Then** it succeeds and returns weather data

#### Scenario: Weather tool called missing lat returns error
**Given** a weather tool
**When** `execute({"longitude": -3.70})` is called (no latitude)
**Then** it returns `Err(ToolError::InvalidArguments)`

### Requirement: geocode tool (split from geo)

La tool `geocode` SHALL exponer únicamente el parámetro `query` para resolver nombres de lugar a coordenadas.

#### Scenario: geocode tool has only query parameter
**Given** the geocode tool definition
**When** `parameters()` is called
**Then** `required` contains `["query"]`
**And** `properties` only has `query` (string)

#### Scenario: geocode resolves a city name
**Given** a geocode tool
**When** `execute({"query": "Madrid"})` is called
**Then** it returns coordinates with lat ~40.41 and lon ~-3.70

### Requirement: reverse_geocode tool (split from geo)

La tool `reverse_geocode` SHALL requerir `latitude` y `longitude` para resolver coordenadas a una dirección.

#### Scenario: reverse_geocode tool has lat/lon parameters
**Given** the reverse_geocode tool definition
**When** `parameters()` is called
**Then** `required` contains `["latitude", "longitude"]`

### Requirement: search_places tool (split from geo)

La tool `search_places` SHALL requerir `query`, `latitude` y `longitude` para buscar lugares cercanos.

#### Scenario: search_places tool has query + lat/lon + radius
**Given** the search_places tool definition
**When** `parameters()` is called
**Then** `required` contains `["query", "latitude", "longitude"]`