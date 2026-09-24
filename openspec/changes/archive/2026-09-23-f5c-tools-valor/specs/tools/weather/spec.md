# Weather Tool

## Contracts

```rust
pub struct WeatherTool {
    db: Arc<Mutex<Connection>>,
    api_key: String,
}

// OpenWeather API response (simplified)
pub struct WeatherResponse {
    pub temperature: f64,        // °C
    pub feels_like: f64,         // °C
    pub humidity: u32,           // %
    pub pressure: u32,           // hPa
    pub description: String,     // "cielo claro"
    pub icon: String,            // "01d"
    pub wind_speed: f64,         // m/s
    pub wind_direction: u32,     // degrees
    pub visibility: u32,         // meters
    pub sunrise: String,         // ISO 8601
    pub sunset: String,          // ISO 8601
    pub date: String,            // ISO 8601 date
}
```

## Scenarios

### Happy path: get_weather with coordinates
**Given** a valid `OPENWEATHER_API_KEY` is configured  
**When** the user calls `get_weather` with `{ "latitude": 40.4168, "longitude": -3.7038 }`  
**Then** the tool returns current weather data including temperature, humidity, description, wind

### Happy path: get_weather with date
**Given** a valid `OPENWEATHER_API_KEY` is configured  
**When** the user calls `get_weather` with `{ "latitude": 40.4168, "longitude": -3.7038, "date": "2026-09-25" }`  
**Then** the tool returns forecast weather data for that date

### Error: missing coordinates
**Given** no API key or invalid coordinates  
**When** the user calls `get_weather` without `latitude` or `longitude`  
**Then** the tool returns `ToolError::InvalidArguments`

### Error: API failure
**Given** the OpenWeather API is unreachable  
**When** the user calls `get_weather` with valid coordinates  
**Then** the tool returns `ToolError::ExecutionError` with the API error message

### Permission
**Given** the weather tool  
**Then** its permission level is `NoConfirm` (read-only, no side effects)