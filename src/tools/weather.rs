use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use rusqlite::Connection;
use serde_json::Value;
use std::sync::{Arc, Mutex};

use crate::tools::permission::Permission;
use crate::tools::r#trait::{Tool, ToolError, ToolResult};

/// Parse a date string, accepting both ISO 8601 datetime and date-only YYYY-MM-DD formats.
///
/// - `"2026-09-25T12:00:00Z"` → parsed directly as `DateTime<Utc>`
/// - `"2026-09-25"` → interpreted as noon UTC on that day
/// - Any other format → returns `None`
fn parse_forecast_date(date_str: &str) -> Option<DateTime<Utc>> {
    date_str.parse::<DateTime<Utc>>().ok().or_else(|| {
        // Try parsing as YYYY-MM-DD (date only), defaulting to noon UTC
        NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .ok()
            .and_then(|d| d.and_hms_opt(12, 0, 0))
            .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
    })
}

pub struct WeatherTool {
    #[allow(dead_code)]
    db: Arc<Mutex<Connection>>,
    api_key: String,
    client: reqwest::Client,
}

impl WeatherTool {
    pub fn new(db: Arc<Mutex<Connection>>, api_key: String) -> Self {
        Self {
            db,
            api_key,
            client: reqwest::Client::new(),
        }
    }

    async fn get_weather(&self, args: Value) -> Result<ToolResult, ToolError> {
        let lat = args
            .get("latitude")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::InvalidArguments("Missing or invalid latitude".into()))?;
        let lon = args
            .get("longitude")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::InvalidArguments("Missing or invalid longitude".into()))?;

        let date = args.get("date").and_then(|v| v.as_str());

        if let Some(date_str) = date {
            self.get_forecast(lat, lon, date_str).await
        } else {
            self.get_current_weather(lat, lon).await
        }
    }

    async fn get_current_weather(&self, lat: f64, lon: f64) -> Result<ToolResult, ToolError> {
        let url = format!(
            "https://api.openweathermap.org/data/2.5/weather?lat={}&lon={}&appid={}&units=metric&lang=es",
            lat, lon, self.api_key
        );

        let resp = self.client.get(&url).send().await.map_err(|e| {
            ToolError::ExecutionError(format!("Failed to call OpenWeather API: {}", e))
        })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ToolError::ExecutionError(format!(
                "OpenWeather API returned {}: {}",
                status, body
            )));
        }

        let json: Value = resp.json().await.map_err(|e| {
            ToolError::ExecutionError(format!("Failed to parse OpenWeather response: {}", e))
        })?;

        Ok(ToolResult {
            success: true,
            data: json,
            message: None,
        })
    }

    async fn get_forecast(
        &self,
        lat: f64,
        lon: f64,
        date_str: &str,
    ) -> Result<ToolResult, ToolError> {
        let url = format!(
            "https://api.openweathermap.org/data/2.5/forecast?lat={}&lon={}&appid={}&units=metric&lang=es",
            lat, lon, self.api_key
        );

        let resp = self.client.get(&url).send().await.map_err(|e| {
            ToolError::ExecutionError(format!("Failed to call OpenWeather API: {}", e))
        })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ToolError::ExecutionError(format!(
                "OpenWeather API returned {}: {}",
                status, body
            )));
        }

        let json: Value = resp.json().await.map_err(|e| {
            ToolError::ExecutionError(format!("Failed to parse OpenWeather response: {}", e))
        })?;

        let target_date = parse_forecast_date(date_str);

        let list = json
            .get("list")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ToolError::ExecutionError("No forecast data in response".into()))?;

        // Find the forecast entry closest to the target date
        let target = target_date.ok_or_else(|| {
            ToolError::ExecutionError(format!(
                "Invalid date format: '{}'. Use YYYY-MM-DD or ISO 8601.",
                date_str
            ))
        })?;

        let closest = list
            .iter()
            .filter_map(|entry| {
                let dt = entry.get("dt_txt").and_then(|v| v.as_str()).and_then(|s| {
                    // OpenWeather dt_txt format: "2026-09-28 00:00:00" (space-separated, no tz)
                    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                        .ok()
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
                        // Fallback for ISO 8601 format
                        .or_else(|| s.parse::<DateTime<Utc>>().ok())
                })?;
                let diff = (dt - target).num_seconds().abs();
                Some((diff, entry))
            })
            .min_by_key(|(diff, _)| *diff)
            .map(|(_, entry)| entry);

        match closest {
            Some(entry) => Ok(ToolResult {
                success: true,
                data: entry.clone(),
                message: None,
            }),
            None => Err(ToolError::ExecutionError(
                "No forecast entry found for the specified date".into(),
            )),
        }
    }
}

#[async_trait]
impl Tool for WeatherTool {
    fn name(&self) -> &'static str {
        "weather"
    }

    fn description(&self) -> &'static str {
        "Consulta del clima actual o pronóstico para coordenadas geográficas"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["get_weather"]
                },
                "latitude": {
                    "type": "number",
                    "description": "Latitud en grados decimales"
                },
                "longitude": {
                    "type": "number",
                    "description": "Longitud en grados decimales"
                },
                "date": {
                    "type": "string",
                    "description": "Fecha opcional para pronóstico (formato ISO 8601)"
                }
            },
            "required": ["operation"]
        })
    }

    fn permission(&self) -> Permission {
        Permission::NoConfirm
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, ToolError> {
        match args.get("operation").and_then(|v| v.as_str()).unwrap_or("") {
            "get_weather" => self.get_weather(args).await,
            op => Err(ToolError::InvalidArguments(format!(
                "Unknown operation: {}",
                op
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::run_migrations;

    fn setup() -> (Arc<Mutex<Connection>>, WeatherTool) {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let db = Arc::new(Mutex::new(conn));
        let tool = WeatherTool::new(db.clone(), "test-api-key".into());
        (db, tool)
    }

    #[tokio::test]
    async fn test_weather_name_and_description() {
        let (_, tool) = setup();
        assert_eq!(tool.name(), "weather");
        assert_eq!(
            tool.description(),
            "Consulta del clima actual o pronóstico para coordenadas geográficas"
        );
    }

    #[tokio::test]
    async fn test_weather_permission() {
        let (_, tool) = setup();
        assert_eq!(tool.permission(), Permission::NoConfirm);
    }

    #[tokio::test]
    async fn test_weather_parameters_has_operation() {
        let (_, tool) = setup();
        let params = tool.parameters();
        assert_eq!(params["type"], "object");
        let operation = params["properties"]["operation"].as_object().unwrap();
        assert_eq!(operation["type"], "string");
        let enum_values = operation["enum"].as_array().unwrap();
        assert!(enum_values.contains(&serde_json::json!("get_weather")));
    }

    #[tokio::test]
    async fn test_weather_missing_coordinates() {
        let (_, tool) = setup();
        // Missing latitude
        let result = tool
            .execute(serde_json::json!({
                "operation": "get_weather",
                "longitude": -3.7038
            }))
            .await;
        assert!(matches!(result, Err(ToolError::InvalidArguments(_))));

        // Missing longitude
        let result = tool
            .execute(serde_json::json!({
                "operation": "get_weather",
                "latitude": 40.4168
            }))
            .await;
        assert!(matches!(result, Err(ToolError::InvalidArguments(_))));

        // Missing both
        let result = tool
            .execute(serde_json::json!({
                "operation": "get_weather"
            }))
            .await;
        assert!(matches!(result, Err(ToolError::InvalidArguments(_))));
    }

    #[tokio::test]
    async fn test_weather_invalid_operation() {
        let (_, tool) = setup();
        let result = tool
            .execute(serde_json::json!({
                "operation": "nonexistent"
            }))
            .await;
        assert!(matches!(result, Err(ToolError::InvalidArguments(_))));
    }

    // ------------------------------------------------------------------
    // parse_forecast_date tests
    // ------------------------------------------------------------------

    #[test]
    fn test_parse_forecast_date_full_datetime() {
        let dt = parse_forecast_date("2026-09-25T12:00:00Z");
        assert!(dt.is_some(), "Full ISO 8601 datetime should parse");
        assert_eq!(dt.unwrap().to_rfc3339(), "2026-09-25T12:00:00+00:00");
    }

    #[test]
    fn test_parse_forecast_date_only_format() {
        // "2026-09-25" should parse via NaiveDate fallback (noon UTC)
        let dt = parse_forecast_date("2026-09-25");
        assert!(dt.is_some(), "Date-only YYYY-MM-DD should parse");
        assert_eq!(dt.unwrap().to_rfc3339(), "2026-09-25T12:00:00+00:00");
    }

    #[test]
    fn test_parse_forecast_invalid_date_returns_none() {
        let dt = parse_forecast_date("not-a-date");
        assert!(dt.is_none(), "Invalid date string should return None");

        let dt = parse_forecast_date("");
        assert!(dt.is_none(), "Empty string should return None");
    }

    #[test]
    fn test_parse_forecast_alt_format() {
        // "2026-09-25 00:00:00" (common in forecast dt_txt) should NOT parse
        // because it's not ISO 8601 and not YYYY-MM-DD
        let dt = parse_forecast_date("2026-09-25 00:00:00");
        assert!(dt.is_none(), "Space-separated datetime should not parse");
    }

    #[test]
    fn test_parse_forecast_edge_date() {
        // Leap year date
        let dt = parse_forecast_date("2024-02-29");
        assert!(dt.is_some(), "Leap year date should parse");
        assert_eq!(dt.unwrap().to_rfc3339(), "2024-02-29T12:00:00+00:00");
    }
}
