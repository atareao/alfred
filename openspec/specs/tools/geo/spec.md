# tools/geo Specification

## Purpose
Búsqueda de lugares y geocodificación usando Google Places API (New API) y Nominatim (OSM).

## Requirements

### Requirement: Google Places API integration

**Given** un `SearchPlacesTool` 
**When** se ejecuta `search_places` con query, lat, lon y radius
**Then** la tool DEBE leer `google_places_api_key` de la tabla `settings` (SettingsRepo)
**And** DEBE llamar a la Google Places New API (`places:searchNearby` o `places:searchText`)
**And** DEBE incluir `X-Goog-Api-Key` y `X-Goog-FieldMask` en los headers
**And** DEBE devolver los resultados como `ToolResult.data` con el JSON completo de la API

#### Scenario: Búsqueda por texto (searchText)
**Given** un `SearchPlacesTool` con API key en settings DB
**When** se ejecuta con `{"query": "restaurantes en Madrid", "latitude": 40.4168, "longitude": -3.7038}`
**Then** la tool DEBE llamar a `POST https://places.googleapis.com/v1/places:searchText`
**And** DEBE incluir `languageCode: "es"` en el body
**And** DEBE devolver los lugares parseados como `Vec<Place>` y el JSON crudo

#### Scenario: Búsqueda por cercanía (searchNearby)
**Given** un `SearchPlacesTool` con API key en settings DB
**When** se ejecuta con `{"query": "cafe", "latitude": 40.4168, "longitude": -3.7038, "radius": 500}`
**Then** la tool DEBE llamar a `POST https://places.googleapis.com/v1/places:searchNearby`
**And** DEBE incluir `locationRestriction.circle` con center y radius en el body
**And** DEBE devolver los lugares parseados como `Vec<Place>` y el JSON crudo

#### Scenario: Error de API key inválida
**Given** un `SearchPlacesTool` con API key inválida en settings DB
**When** se ejecuta `search_places`
**Then** DEBE retornar `Err(ToolError::ExecutionError)` con mensaje del error HTTP

#### Scenario: Sin API key configurada
**Given** un `SearchPlacesTool` sin API key en settings DB ni en Config/ENV
**When** se ejecuta `search_places`
**Then** DEBE retornar `Err(ToolError::ExecutionError)` indicando que falta la API key

### Requirement: Estructura Place con campos de Google Places

**Given** la respuesta de Google Places API
**When** se deserializa
**Then** DEBE mapear a la estructura `Place` con los siguientes campos:
- `id: Option<String>`
- `display_name: Option<LocalizedText>` (text + language_code)
- `formatted_address: Option<String>`
- `location: Option<LatLng>` (latitude + longitude)
- `rating: Option<f64>`
- `user_rating_count: Option<u64>`
- `price_level: Option<String>`
- `website_uri: Option<String>`
- `national_phone_number: Option<String>`
- `regular_opening_hours: Option<OpeningHours>` (open_now + weekday_descriptions)
- `primary_type: Option<String>`
- `types: Option<Vec<String>>`
- `editorial_summary: Option<LocalizedText>`

#### Scenario: Deserialización de Place completo
**Given** un JSON de respuesta de Google Places con todos los campos
**When** se deserializa a `PlacesResponse`
**Then** `places` contiene `Vec<Place>` con todos los campos mapeados correctamente

#### Scenario: Deserialización con campos nulos
**Given** un JSON de respuesta con campos opcionales ausentes
**When** se deserializa a `PlacesResponse`
**Then** los campos ausentes DEBEN ser `None` (no panic)

### Requirement: FIELD_MASK para minimizar coste

**Given** la constante `FIELD_MASK`
**When** se usa en requests a Google Places
**Then** DEBE incluir: `places.id`, `places.displayName`, `places.formattedAddress`, `places.location`, `places.rating`, `places.userRatingCount`, `places.priceLevel`, `places.websiteUri`, `places.nationalPhoneNumber`, `places.regularOpeningHours`, `places.primaryType`, `places.types`, `places.editorialSummary`

### Requirement: Enlace a Google Maps

**Given** un `SearchPlacesTool` con una lista de `Place`
**When** se llama a `maps_link(places)`
**Then** DEBE generar un URL `https://www.google.com/maps/dir/{lat,lon|lat,lon...}`
**And** DEBE filtrar lugares sin coordenadas
**And** DEBE retornar string vacía si no hay lugares con coordenadas

#### Scenario: Maps link con un solo lugar
**Given** un lugar con coordenadas (40.3520, 18.1715)
**When** se genera maps_link
**Then** retorna `"https://www.google.com/maps/dir/40.35200,18.17150"`

#### Scenario: Maps link con múltiples lugares
**Given** dos lugares con coordenadas
**When** se genera maps_link
**Then** retorna URLs separados por `|`

#### Scenario: Maps link vacío
**Given** lista vacía de lugares
**When** se genera maps_link
**Then** retorna `""`

### Requirement: GeocodeTool (Nominatim)

**Given** un `GeocodeTool`
**When** se ejecuta `geocode` con una dirección
**Then** DEBE llamar a Nominatim API (`/search`)
**And** DEBE devolver coordenadas y dirección formateada

#### Scenario: Geocode exitoso
**Given** un `GeocodeTool`
**When** se ejecuta con `{"query": "Plaza Mayor, Madrid"}`
**Then** DEBE retornar `ToolResult` con `latitude`, `longitude`, `display_name`, `address`

#### Scenario: Geocode sin resultados
**Given** un `GeocodeTool`
**When** se ejecuta con una dirección inexistente
**Then** DEBE retornar `Err(ToolError::NotFound)`

### Requirement: ReverseGeocodeTool (Nominatim)

**Given** un `ReverseGeocodeTool`
**When** se ejecuta `reverse_geocode` con coordenadas
**Then** DEBE llamar a Nominatim API (`/reverse`)
**And** DEBE devolver la dirección formateada

#### Scenario: Reverse geocode exitoso
**Given** un `ReverseGeocodeTool`
**When** se ejecuta con `{"latitude": 40.4155, "longitude": -3.7074}`
**Then** DEBE retornar `ToolResult` con dirección formateada

#### Scenario: Reverse geocode sin resultados
**Given** un `ReverseGeocodeTool`
**When** se ejecuta con coordenadas en medio del océano
**Then** DEBE retornar `Err(ToolError::NotFound)`