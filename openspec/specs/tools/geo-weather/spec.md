# geo/weather tools spec

## Requirement: Weather tool — remove operation, add required lat/lon

### Scenario: Weather tool has no operation parameter
**Given** the weather tool definition
**When** `parameters()` is called
**Then** the schema must NOT have an `operation` property
**And** `required` must contain `["latitude", "longitude"]`

### Scenario: Weather tool called with lat/lon returns data
**Given** a weather tool with a valid API key
**When** `execute({"latitude": 40.41, "longitude": -3.70})` is called
**Then** it succeeds and returns weather data

### Scenario: Weather tool called missing lat returns error
**Given** a weather tool
**When** `execute({"longitude": -3.70})` is called (no latitude)
**Then** it returns `Err(ToolError::InvalidArguments)`

## Requirement: geocode tool (split from geo)

### Scenario: geocode tool has only query parameter
**Given** the geocode tool definition
**When** `parameters()` is called
**Then** `required` contains `["query"]`
**And** `properties` only has `query` (string)

### Scenario: geocode resolves a city name
**Given** a geocode tool
**When** `execute({"query": "Madrid"})` is called
**Then** it returns coordinates with lat ~40.41 and lon ~-3.70

## Requirement: reverse_geocode tool (split from geo)

### Scenario: reverse_geocode tool has lat/lon parameters
**Given** the reverse_geocode tool definition
**When** `parameters()` is called
**Then** `required` contains `["latitude", "longitude"]`

## Requirement: search_places tool (split from geo)

### Scenario: search_places tool has query + lat/lon + radius
**Given** the search_places tool definition
**When** `parameters()` is called
**Then** `required` contains `["query", "latitude", "longitude"]`