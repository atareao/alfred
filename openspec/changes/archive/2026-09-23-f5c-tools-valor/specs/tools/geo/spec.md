# Geo Tool

## Contracts

```rust
pub struct GeoTool {
    db: Arc<Mutex<Connection>>,
}

pub struct GeocodeResult {
    pub latitude: f64,
    pub longitude: f64,
    pub display_name: String,
    pub address: AddressComponents,
}

pub struct AddressComponents {
    pub road: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub postcode: Option<String>,
}

pub struct Place {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub category: String,       // "restaurant", "cafe", "park", etc.
    pub address: Option<String>,
    pub distance_m: Option<f64>,
}
```

## Scenarios

### Happy path: geocode address
**Given** a valid address string  
**When** the user calls `geocode` with `{ "query": "Plaza Mayor, Madrid" }`  
**Then** the tool returns `{ "latitude": 40.4155, "longitude": -3.7074, "display_name": "Plaza Mayor, ..." }`

### Happy path: reverse geocode
**Given** valid coordinates  
**When** the user calls `reverse_geocode` with `{ "latitude": 40.4168, "longitude": -3.7038 }`  
**Then** the tool returns the address components for those coordinates

### Happy path: search places
**Given** a query and coordinates  
**When** the user calls `search_places` with `{ "query": "café", "latitude": 40.4168, "longitude": -3.7038, "radius": 1000 }`  
**Then** the tool returns nearby places matching the query within the radius

### Error: missing query for geocode
**When** the user calls `geocode` without `query`  
**Then** the tool returns `ToolError::InvalidArguments`

### Error: missing coordinates for reverse_geocode
**When** the user calls `reverse_geocode` without `latitude` or `longitude`  
**Then** the tool returns `ToolError::InvalidArguments`

### Error: Nominatim API failure
**Given** Nominatim is unreachable  
**When** the user calls `geocode` with a valid query  
**Then** the tool returns `ToolError::ExecutionError`

### Permission
**Given** the geo tool  
**Then** its permission level is `NoConfirm` (read-only)