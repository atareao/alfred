/// Spanish day-of-week names (ISO weekday: 1 = Monday … 7 = Sunday).
pub const DIAS: [&str; 7] = [
    "lunes",
    "martes",
    "miércoles",
    "jueves",
    "viernes",
    "sábado",
    "domingo",
];

/// Spanish month names.
pub const MESES: [&str; 12] = [
    "enero",
    "febrero",
    "marzo",
    "abril",
    "mayo",
    "junio",
    "julio",
    "agosto",
    "septiembre",
    "octubre",
    "noviembre",
    "diciembre",
];

/// Spanish time-of-day phrases keyed by hour.
pub fn momento_del_dia(hora: u32) -> &'static str {
    match hora {
        0..=5 => "de la madrugada",
        6..=11 => "de la mañana",
        12..=20 => "de la tarde",
        _ => "de la noche",
    }
}

/// Parse an ISO‑8601 UTC timestamp and return a human‑readable Spanish string.
///
/// Returns `None` on parse failure so callers can fall back gracefully.
pub fn format_browser_timestamp(iso: &str, tz: &str) -> Option<String> {
    use chrono::{Datelike, NaiveDateTime, Timelike};
    use chrono_tz::Tz;
    use std::str::FromStr;

    // Accept both "2026-09-24T08:00:00Z" and "2026-09-26T10:00:00.000Z"
    let naive =
        NaiveDateTime::parse_from_str(iso.trim_end_matches('Z'), "%Y-%m-%dT%H:%M:%S%.f").ok()?;

    let utc_dt: chrono::DateTime<chrono::Utc> =
        chrono::DateTime::from_naive_utc_and_offset(naive, chrono::Utc);

    // Convert to user's timezone
    let tz = Tz::from_str(tz).ok()?;
    let dt = utc_dt.with_timezone(&tz);

    let wd = dt.format("%u").to_string().parse::<usize>().ok()?; // 1–7
    let day_name = DIAS.get(wd - 1)?;
    let month_name = MESES.get((dt.month0()) as usize)?;
    let momento = momento_del_dia(dt.hour());

    Some(format!(
        "Hoy es {}, {} de {} de {}, son las {}:{:02} {}",
        day_name,
        dt.day(),
        month_name,
        dt.year(),
        dt.hour(),
        dt.minute(),
        momento,
    ))
}

/// Format the current UTC time in the given timezone as a human‑readable
/// Spanish string.  Returns the same format as [`format_browser_timestamp`]
/// but computed from the current system clock.
pub fn format_time_now(timezone: &str) -> String {
    use chrono::{Datelike, Timelike};
    use chrono_tz::Tz;
    use std::str::FromStr;

    let utc_now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();

    // Fall back to UTC if the timezone string is invalid.
    let tz = Tz::from_str(timezone).unwrap_or(chrono_tz::UTC);
    let dt = utc_now.with_timezone(&tz);

    let wd = dt.format("%u").to_string().parse::<usize>().unwrap_or(1); // 1–7
    let day_name = DIAS[wd - 1];
    let month_name = MESES[dt.month0() as usize];
    let momento = momento_del_dia(dt.hour());

    format!(
        "Hoy es {}, {} de {} de {}, son las {}:{:02} {}",
        day_name,
        dt.day(),
        month_name,
        dt.year(),
        dt.hour(),
        dt.minute(),
        momento,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_momento_del_dia_madrugada() {
        assert_eq!(momento_del_dia(0), "de la madrugada");
        assert_eq!(momento_del_dia(5), "de la madrugada");
    }

    #[test]
    fn test_momento_del_dia_manana() {
        assert_eq!(momento_del_dia(6), "de la mañana");
        assert_eq!(momento_del_dia(11), "de la mañana");
    }

    #[test]
    fn test_momento_del_dia_tarde() {
        assert_eq!(momento_del_dia(12), "de la tarde");
        assert_eq!(momento_del_dia(20), "de la tarde");
    }

    #[test]
    fn test_momento_del_dia_noche() {
        assert_eq!(momento_del_dia(21), "de la noche");
        assert_eq!(momento_del_dia(23), "de la noche");
    }

    #[test]
    fn test_dias_length() {
        assert_eq!(DIAS.len(), 7);
    }

    #[test]
    fn test_meses_length() {
        assert_eq!(MESES.len(), 12);
    }

    #[test]
    fn test_format_browser_timestamp_valid() {
        let result = format_browser_timestamp("2026-09-26T08:00:00Z", "Europe/Madrid");
        assert!(result.is_some());
        let s = result.unwrap();
        // 2026-09-26 is a Saturday → sábado
        assert!(s.contains("sábado"));
        assert!(s.contains("26"));
        assert!(s.contains("septiembre"));
        assert!(s.contains("2026"));
        // 08:00 UTC → 10:00 CEST → mañana
        assert!(s.contains("10:00"));
        assert!(s.contains("de la mañana"));
    }

    #[test]
    fn test_format_browser_timestamp_with_millis() {
        let result = format_browser_timestamp("2026-09-26T10:00:00.000Z", "Europe/Madrid");
        assert!(result.is_some());
        let s = result.unwrap();
        assert!(s.contains("12:00"));
        assert!(s.contains("de la tarde"));
    }

    #[test]
    fn test_format_browser_timestamp_invalid() {
        assert!(format_browser_timestamp("not-a-date", "Europe/Madrid").is_none());
        assert!(format_browser_timestamp("2026-09-26T08:00:00Z", "Invalid/Zone").is_none());
    }

    #[test]
    fn test_format_time_now_returns_string() {
        let s = format_time_now("Europe/Madrid");
        assert!(s.starts_with("Hoy es "));
        assert!(s.contains("son las "));
    }

    #[test]
    fn test_format_time_now_invalid_tz_falls_back() {
        // Invalid timezone should fall back to UTC without panicking.
        let s = format_time_now("Bad/Zone");
        assert!(s.starts_with("Hoy es "));
    }
}
