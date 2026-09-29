use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;

/// Reverse‑geocode coordinates via Nominatim and return a short location name.
///
/// Results are cached in memory for 5 minutes, keyed by coordinates rounded
/// to 2 decimal places (≈1 km precision).
///
/// Returns `None` on any error (network, parse, no results) so callers can
/// fall back to showing raw coordinates.
pub async fn reverse_geocode(lat: f64, lon: f64) -> Option<String> {
    /// Cache key: (lat×2000, lon×2000) rounded to integers (~55 m precision).
    type CacheKey = (i32, i32);
    /// Cache value: (when cached, location name).
    type CacheVal = (Instant, String);

    static CACHE: LazyLock<Mutex<HashMap<CacheKey, CacheVal>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));

    let key: CacheKey = ((lat * 2000.0).round() as i32, (lon * 2000.0).round() as i32);

    // Check cache (5-minute TTL)
    {
        let cache = CACHE.lock().unwrap();
        if let Some((cached_at, name)) = cache.get(&key) {
            if cached_at.elapsed() < std::time::Duration::from_secs(300) {
                return Some(name.clone());
            }
        }
    }

    let url = format!(
        "https://nominatim.openstreetmap.org/reverse?lat={}&lon={}&format=json&addressdetails=1",
        lat, lon
    );

    let client = reqwest::Client::builder()
        .user_agent("Valet/0.5 (valet@atareao.es)")
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .ok()?;

    let resp = client.get(&url).send().await.ok()?;
    let body: serde_json::Value = resp.json().await.ok()?;

    if body.get("error").is_some() {
        return None;
    }

    let addr = body.get("address")?;
    // Build a compact location string: street, town/city, region, country
    let mut parts: Vec<String> = Vec::new();

    // Street + house number
    if let Some(road) = addr.get("road").and_then(|v| v.as_str()) {
        let s = match addr.get("house_number").and_then(|v| v.as_str()) {
            Some(n) => format!("{} {}", road, n),
            None => road.to_string(),
        };
        parts.push(s);
    }

    // Town / city / village / hamlet (first match wins, dedup against street)
    for key in ["town", "city", "village", "hamlet"] {
        if let Some(v) = addr.get(key).and_then(|v| v.as_str()) {
            if parts.last().map(|l| l != v).unwrap_or(true) {
                parts.push(v.to_string());
                break;
            }
        }
    }

    // State, country
    for key in ["state", "country"] {
        if let Some(v) = addr.get(key).and_then(|v| v.as_str()) {
            if parts.last().map(|l| l != v).unwrap_or(true) {
                parts.push(v.to_string());
            }
        }
    }

    if parts.is_empty() {
        None
    } else {
        // Deduplicate consecutive identical parts (e.g. same city and village)
        let mut dedup: Vec<&str> = Vec::new();
        for p in &parts {
            if dedup.last().map(|&l| l != p.as_str()).unwrap_or(true) {
                dedup.push(p.as_str());
            }
        }
        let result = dedup.join(", ");

        // Store in cache
        {
            let mut cache = CACHE.lock().unwrap();
            cache.insert(key, (Instant::now(), result.clone()));
        }

        Some(result)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_cache_key_precision() {
        // Verify the rounding gives consistent keys for nearby coordinates
        let key_a: (i32, i32) = (
            (39.4733f64 * 2000.0).round() as i32,
            (-0.3755f64 * 2000.0).round() as i32,
        );
        let key_b: (i32, i32) = (
            (39.4734f64 * 2000.0).round() as i32,
            (-0.3756f64 * 2000.0).round() as i32,
        );
        assert_eq!(
            key_a, key_b,
            "Coordinates within ~55 m should produce the same cache key"
        );
    }
}
