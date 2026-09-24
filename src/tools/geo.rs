use async_trait::async_trait;
use rusqlite::Connection;
use serde_json::Value;
use std::sync::{Arc, Mutex};

use crate::tools::permission::Permission;
use crate::tools::r#trait::{Tool, ToolError, ToolResult};

/// Tool for geolocation: direct/reverse geocoding and nearby place search.
pub struct GeoTool {
    #[allow(dead_code)]
    db: Arc<Mutex<Connection>>,
    client: reqwest::Client,
}

impl GeoTool {
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Alfred/1.0")
            .build()
            .expect("Failed to build reqwest Client");
        Self { db, client }
    }

    /// Create a GeoTool with a custom reqwest Client (used in tests).
    #[cfg(test)]
    fn with_client(db: Arc<Mutex<Connection>>, client: reqwest::Client) -> Self {
        Self { db, client }
    }

    /// Geocode a query string via Nominatim.
    async fn geocode(&self, args: Value) -> Result<ToolResult, ToolError> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing query".into()))?;

        let url = format!(
            "https://nominatim.openstreetmap.org/search?q={}&format=json&limit=1&addressdetails=1",
            urlencoding(query)
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Nominatim request failed: {e}")))?;

        let body: Value = resp
            .json()
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Nominatim parse failed: {e}")))?;

        let results = body
            .as_array()
            .ok_or_else(|| ToolError::ExecutionError("Unexpected Nominatim response".into()))?;

        let first = results
            .first()
            .ok_or_else(|| ToolError::NotFound("No results found".into()))?;

        Ok(ToolResult {
            success: true,
            data: Self::nominatim_to_geo_result(first),
            message: None,
        })
    }

    /// Reverse geocode coordinates via Nominatim.
    async fn reverse_geocode(&self, args: Value) -> Result<ToolResult, ToolError> {
        let lat = args
            .get("latitude")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::InvalidArguments("Missing or invalid latitude".into()))?;
        let lon = args
            .get("longitude")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::InvalidArguments("Missing or invalid longitude".into()))?;

        let url = format!(
            "https://nominatim.openstreetmap.org/reverse?lat={}&lon={}&format=json&addressdetails=1",
            lat, lon
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Nominatim request failed: {e}")))?;

        let body: Value = resp
            .json()
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Nominatim parse failed: {e}")))?;

        if body.get("error").is_some() {
            return Err(ToolError::NotFound(
                body["error"]
                    .as_str()
                    .unwrap_or("No results found")
                    .to_string(),
            ));
        }

        Ok(ToolResult {
            success: true,
            data: Self::nominatim_to_geo_result(&body),
            message: None,
        })
    }

    /// Search places near a location via Overpass API.
    async fn search_places(&self, args: Value) -> Result<ToolResult, ToolError> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing query".into()))?;
        let lat = args
            .get("latitude")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::InvalidArguments("Missing or invalid latitude".into()))?;
        let lon = args
            .get("longitude")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::InvalidArguments("Missing or invalid longitude".into()))?;
        let radius = args.get("radius").and_then(|v| v.as_u64()).unwrap_or(1000);

        let overpass_query = format!(
            "[out:json];(node(around:{radius},{lat},{lon})[name~\"{regex}\",i];way(around:{radius},{lat},{lon})[name~\"{regex}\",i];);out center 10;",
            radius = radius,
            lat = lat,
            lon = lon,
            regex = overpass_escape(query)
        );

        let resp = self
            .client
            .post("https://overpass-api.de/api/interpreter")
            .body(overpass_query)
            .send()
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Overpass request failed: {e}")))?;

        let body: Value = resp
            .json()
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Overpass parse failed: {e}")))?;

        let elements = body["elements"]
            .as_array()
            .ok_or_else(|| ToolError::ExecutionError("Unexpected Overpass response".into()))?;

        let mut places = Vec::new();
        for elem in elements {
            let name = elem["tags"]["name"].as_str().unwrap_or("Unknown");
            let cat = elem["tags"]["amenity"]
                .as_str()
                .or_else(|| elem["tags"]["shop"].as_str())
                .or_else(|| elem["tags"]["leisure"].as_str())
                .or_else(|| elem["tags"]["tourism"].as_str())
                .unwrap_or("other");

            // Overpass returns lat/lon at top level for nodes, or in center for ways
            let (place_lat, place_lon) = elem
                .get("lat")
                .and_then(|v| v.as_f64())
                .zip(elem.get("lon").and_then(|v| v.as_f64()))
                .or_else(|| {
                    elem["center"]
                        .get("lat")
                        .and_then(|v| v.as_f64())
                        .zip(elem["center"].get("lon").and_then(|v| v.as_f64()))
                })
                .unwrap_or((lat, lon));

            let address = elem["tags"]["addr:street"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let housenumber = elem["tags"]["addr:housenumber"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let address_display = if address.is_empty() && housenumber.is_empty() {
                String::new()
            } else {
                format!("{} {}", housenumber, address).trim().to_string()
            };

            let distance = haversine_distance(lat, lon, place_lat, place_lon);

            places.push(serde_json::json!({
                "name": name,
                "latitude": place_lat,
                "longitude": place_lon,
                "category": cat,
                "address": address_display,
                "distance_m": (distance * 1000.0).round() as u64,
            }));
        }

        Ok(ToolResult {
            success: true,
            data: serde_json::json!({"places": places}),
            message: Some(format!("Found {} places", places.len())),
        })
    }

    /// Convert a Nominatim result JSON object into the standard geo result format.
    fn nominatim_to_geo_result(item: &Value) -> Value {
        let address = item.get("address").and_then(|a| a.as_object()).map(|addr| {
            serde_json::json!({
                "road": addr.get("road").or_else(|| addr.get("pedestrian")).or_else(|| addr.get("footway")).and_then(|v| v.as_str()).unwrap_or(""),
                "city": addr.get("city").or_else(|| addr.get("town")).or_else(|| addr.get("village")).or_else(|| addr.get("municipality")).and_then(|v| v.as_str()).unwrap_or(""),
                "state": addr.get("state").and_then(|v| v.as_str()).unwrap_or(""),
                "country": addr.get("country").and_then(|v| v.as_str()).unwrap_or(""),
                "postcode": addr.get("postcode").and_then(|v| v.as_str()).unwrap_or(""),
            })
        }).unwrap_or_default();

        serde_json::json!({
            "latitude": item["lat"].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0),
            "longitude": item["lon"].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0),
            "display_name": item["display_name"].as_str().unwrap_or(""),
            "address": address,
        })
    }
}

#[async_trait]
impl Tool for GeoTool {
    fn name(&self) -> &'static str {
        "geo"
    }

    fn description(&self) -> &'static str {
        "Geolocalización: geocoding directo e inverso, y búsqueda de lugares cercanos"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["geocode", "reverse_geocode", "search_places"],
                    "description": "Operación a realizar"
                },
                "query": {
                    "type": "string",
                    "description": "Dirección o lugar a geocodificar (para geocode / search_places)"
                },
                "latitude": {
                    "type": "number",
                    "description": "Latitud (para reverse_geocode / search_places)"
                },
                "longitude": {
                    "type": "number",
                    "description": "Longitud (para reverse_geocode / search_places)"
                },
                "radius": {
                    "type": "integer",
                    "description": "Radio de búsqueda en metros (para search_places, default: 1000)"
                }
            },
            "required": ["operation"]
        })
    }

    fn permission(&self) -> Permission {
        Permission::NoConfirm
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, ToolError> {
        let operation = args.get("operation").and_then(|v| v.as_str()).unwrap_or("");

        match operation {
            "geocode" => self.geocode(args).await,
            "reverse_geocode" => self.reverse_geocode(args).await,
            "search_places" => self.search_places(args).await,
            _ => Err(ToolError::InvalidArguments(format!(
                "Unknown operation: {operation}"
            ))),
        }
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// URL-encode a string for use in a query parameter.
fn urlencoding(s: &str) -> String {
    let mut encoded = String::new();
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            b' ' => encoded.push_str("%20"),
            _ => {
                encoded.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    encoded
}

/// Escape a string for use inside an Overpass regex (name~"...").
fn overpass_escape(s: &str) -> String {
    // Only escape backslash and double-quote — everything else is fine inside Overpass regex
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Haversine distance in kilometres between two lat/lon points.
fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6371.0; // Earth radius in km
    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();
    let a = (d_lat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();
    r * c
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> GeoTool {
        let conn = Connection::open_in_memory().unwrap();
        let db = Arc::new(Mutex::new(conn));
        // Use a no-op client that won't actually make requests
        let client = reqwest::Client::builder()
            .user_agent("Alfred/1.0")
            .build()
            .unwrap();
        GeoTool::with_client(db, client)
    }

    #[tokio::test]
    async fn test_geo_name_and_description() {
        let tool = setup();
        assert_eq!(tool.name(), "geo");
        assert_eq!(
            tool.description(),
            "Geolocalización: geocoding directo e inverso, y búsqueda de lugares cercanos"
        );
    }

    #[tokio::test]
    async fn test_geo_permission() {
        let tool = setup();
        assert_eq!(tool.permission(), Permission::NoConfirm);
    }

    #[tokio::test]
    async fn test_geo_parameters_has_operations() {
        let tool = setup();
        let params = tool.parameters();
        let ops = params["properties"]["operation"]["enum"]
            .as_array()
            .unwrap();
        let names: Vec<&str> = ops.iter().map(|v| v.as_str().unwrap()).collect();
        assert_eq!(names, vec!["geocode", "reverse_geocode", "search_places"]);
    }

    #[tokio::test]
    async fn test_geocode_missing_query() {
        let tool = setup();
        let result = tool
            .execute(serde_json::json!({"operation": "geocode"}))
            .await;
        assert!(matches!(result, Err(ToolError::InvalidArguments(_))));
    }

    #[tokio::test]
    async fn test_reverse_geocode_missing_coordinates() {
        let tool = setup();
        let result = tool
            .execute(serde_json::json!({"operation": "reverse_geocode"}))
            .await;
        assert!(matches!(result, Err(ToolError::InvalidArguments(_))));
    }

    #[tokio::test]
    async fn test_search_places_missing_args() {
        let tool = setup();
        // Missing query
        let result = tool
            .execute(serde_json::json!({"operation": "search_places", "latitude": 40.41, "longitude": -3.70}))
            .await;
        assert!(matches!(result, Err(ToolError::InvalidArguments(_))));

        // Missing latitude
        let result = tool
            .execute(serde_json::json!({"operation": "search_places", "query": "cafe", "longitude": -3.70}))
            .await;
        assert!(matches!(result, Err(ToolError::InvalidArguments(_))));

        // Missing longitude
        let result = tool
            .execute(serde_json::json!({"operation": "search_places", "query": "cafe", "latitude": 40.41}))
            .await;
        assert!(matches!(result, Err(ToolError::InvalidArguments(_))));
    }

    #[tokio::test]
    async fn test_invalid_operation() {
        let tool = setup();
        let result = tool
            .execute(serde_json::json!({"operation": "nonexistent"}))
            .await;
        assert!(matches!(result, Err(ToolError::InvalidArguments(_))));
    }

    // -----------------------------------------------------------------------
    // Unit tests for helper functions
    // -----------------------------------------------------------------------

    #[test]
    fn test_urlencoding_encodes_spaces_and_specials() {
        assert_eq!(urlencoding("hello world"), "hello%20world");
        assert_eq!(urlencoding("a/b"), "a%2Fb");
        assert_eq!(urlencoding("alphanumeric123"), "alphanumeric123");
    }

    #[test]
    fn test_overpass_escape_escapes_quotes() {
        assert_eq!(overpass_escape("cafe"), "cafe");
        assert_eq!(overpass_escape("bar \"la\" la"), "bar \\\"la\\\" la");
    }

    #[test]
    fn test_haversine_distance_known_points() {
        // Madrid -> Barcelona: roughly 505 km
        let d = haversine_distance(40.4168, -3.7038, 41.3874, 2.1686);
        let diff = (d - 505.0).abs();
        assert!(diff < 10.0, "Expected ~505 km, got {d} km");
    }

    #[test]
    fn test_haversine_distance_zero() {
        let d = haversine_distance(40.0, -3.0, 40.0, -3.0);
        assert_eq!(d, 0.0);
    }

    #[test]
    fn test_nominatim_to_geo_result_parses_string_coords() {
        let item = serde_json::json!({
            "lat": "40.4155",
            "lon": "-3.7074",
            "display_name": "Plaza Mayor, Madrid, España",
            "address": {
                "road": "Plaza Mayor",
                "city": "Madrid",
                "state": "Comunidad de Madrid",
                "country": "España",
                "postcode": "28012"
            }
        });
        let result = GeoTool::nominatim_to_geo_result(&item);
        assert_eq!(result["latitude"].as_f64(), Some(40.4155));
        assert_eq!(result["longitude"].as_f64(), Some(-3.7074));
        assert_eq!(result["display_name"], "Plaza Mayor, Madrid, España");
        assert_eq!(result["address"]["city"], "Madrid");
        assert_eq!(result["address"]["postcode"], "28012");
    }
}
