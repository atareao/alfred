### Requirement: `get_current_time` tool

**Given** un `CurrentTimeTool`
**When** se ejecuta `get_current_time`
**Then** DEBE leer `timezone` de `SettingsRepo` (fallback: `Europe/Madrid`)
**And** DEBE computar la hora actual con `chrono::Utc::now()` convertida a la timezone
**And** DEBE formatear usando el mismo formato que `format_browser_timestamp` produce
**And** DEBE devolver una string como: `"Hoy es sábado, 26 de septiembre de 2026, son las 10:15 de la mañana. Zona horaria: Europe/Madrid."`
**And** DEBE tener permisos `NoConfirm`

#### Scenario: Hora actual con timezone configurada
**Given** settings tiene `timezone = Europe/Madrid`
**When** se ejecuta `get_current_time`
**Then** DEBE devolver la fecha/hora actual en formato español con zona horaria

#### Scenario: Sin timezone configurada (fallback UTC)
**Given** settings NO tiene `timezone`
**When** se ejecuta `get_current_time`
**Then** DEBE usar UTC como fallback
**And** DEBE devolver la fecha/hora actual en UTC

### Requirement: `get_current_location` tool

**Given** un `CurrentLocationTool`
**When** se ejecuta `get_current_location`
**Then** DEBE leer `latitude`, `longitude` de `SettingsRepo`
**And** SI tiene coordenadas DEBE hacer reverse-geocoding (Nominatim) para obtener dirección completa con calle
**And** DEBE devolver: `"Calle Mayor 3, Silla, Valencia, España (39.3600, -0.4100)."`
**And** SI no tiene coordenadas DEBE devolver: `"Ubicación no configurada."`
**And** DEBE tener permisos `NoConfirm`

#### Scenario: Con coordenadas y reverse-geocode exitoso
**Given** settings tiene `latitude=39.36`, `longitude=-0.41`
**When** se ejecuta `get_current_location`
**Then** DEBE llamar a Nominatim reverse
**And** DEBE devolver dirección + coordenadas

#### Scenario: Sin coordenadas configuradas
**Given** settings NO tiene `latitude` ni `longitude`
**When** se ejecuta `get_current_location`
**Then** DEBE devolver `"Ubicación no configurada."`

### Requirement: Persistir browser_context en settings

**Given** el handler `POST /api/chat/stream`
**When** recibe `browser_context` con `timezone`, `latitude`, `longitude`, `location_name`
**Then** DEBE persistir estos valores en `SettingsRepo`
**And** NO DEBE fallar si `browser_context` es `None`

### Requirement: Formateo compartido (time_format)

**Given** el módulo `src/tools/time_format.rs`
**When** se usa desde `agent.rs` o desde `CurrentTimeTool`
**Then** DEBE exportar `format_browser_timestamp` con la misma firma y comportamiento
**And** DEBE exportar `format_time_now(timezone: &str)` que computa `Utc::now()` y lo formatea igual

### Requirement: Reverse-geocode compartido (geo_utils)

**Given** el módulo `src/tools/geo_utils.rs`
**When** se usa desde `agent.rs` o desde `CurrentLocationTool`
**Then** DEBE exportar `reverse_geocode(lat: f64, lon: f64) -> Option<String>` con cache de 5 min