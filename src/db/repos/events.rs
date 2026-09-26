use chrono::{
    DateTime, Datelike, Duration, FixedOffset, NaiveDate, NaiveDateTime, Timelike, Weekday,
};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub profile_id: String,
    pub title: String,
    pub description: Option<String>,
    pub start_time: String,
    pub end_time: String,
    pub location: Option<String>,
    pub scope: String,
    pub category: String,
    pub all_day: bool,
    pub rrule: Option<String>,
    pub reminder_minutes_before: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct EventsRepo;

impl EventsRepo {
    pub async fn create(pool: &SqlitePool, event: &Event) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO events (id, profile_id, title, description, start_time, end_time, location, scope, category, all_day, rrule, reminder_minutes_before, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        )
        .bind(&event.id)
        .bind(&event.profile_id)
        .bind(&event.title)
        .bind(&event.description)
        .bind(&event.start_time)
        .bind(&event.end_time)
        .bind(&event.location)
        .bind(&event.scope)
        .bind(&event.category)
        .bind(event.all_day)
        .bind(&event.rrule)
        .bind(event.reminder_minutes_before)
        .bind(&event.created_at)
        .bind(&event.updated_at)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Event>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, profile_id, title, description, start_time, end_time, location, scope, category, all_day, rrule, reminder_minutes_before, created_at, updated_at
             FROM events WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|r| Event {
            id: r.get(0),
            profile_id: r.get(1),
            title: r.get(2),
            description: r.get(3),
            start_time: r.get(4),
            end_time: r.get(5),
            location: r.get(6),
            scope: r.get(7),
            category: r.get(8),
            all_day: r.get(9),
            rrule: r.get(10),
            reminder_minutes_before: r.get(11),
            created_at: r.get(12),
            updated_at: r.get(13),
        }))
    }

    /// Delete an event by id. Returns true if an event was deleted, false if not found.
    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM events WHERE id = ?1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn list_by_category(
        pool: &SqlitePool,
        profile_id: &str,
        category: &str,
    ) -> Result<Vec<Event>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, profile_id, title, description, start_time, end_time, location, scope, category, all_day, rrule, reminder_minutes_before, created_at, updated_at
             FROM events WHERE profile_id = ?1 AND category = ?2
             ORDER BY start_time ASC",
        )
        .bind(profile_id)
        .bind(category)
        .fetch_all(pool)
        .await?;

        let events: Vec<Event> = rows
            .iter()
            .map(|r| Event {
                id: r.get(0),
                profile_id: r.get(1),
                title: r.get(2),
                description: r.get(3),
                start_time: r.get(4),
                end_time: r.get(5),
                location: r.get(6),
                scope: r.get(7),
                category: r.get(8),
                all_day: r.get(9),
                rrule: r.get(10),
                reminder_minutes_before: r.get(11),
                created_at: r.get(12),
                updated_at: r.get(13),
            })
            .collect();

        Ok(events)
    }

    pub async fn list_by_date_range(
        pool: &SqlitePool,
        profile_id: &str,
        start: &str,
        end: &str,
    ) -> Result<Vec<Event>, sqlx::Error> {
        // Non-recurring events within the range
        let non_recurring = sqlx::query(
            "SELECT id, profile_id, title, description, start_time, end_time, location, scope, category, all_day, rrule, reminder_minutes_before, created_at, updated_at
             FROM events WHERE profile_id = ?1 AND rrule IS NULL AND start_time >= ?2 AND end_time <= ?3
             ORDER BY start_time ASC",
        )
        .bind(profile_id)
        .bind(start)
        .bind(end)
        .fetch_all(pool)
        .await?;

        let mut events: Vec<Event> = non_recurring
            .iter()
            .map(|r| Event {
                id: r.get(0),
                profile_id: r.get(1),
                title: r.get(2),
                description: r.get(3),
                start_time: r.get(4),
                end_time: r.get(5),
                location: r.get(6),
                scope: r.get(7),
                category: r.get(8),
                all_day: r.get(9),
                rrule: r.get(10),
                reminder_minutes_before: r.get(11),
                created_at: r.get(12),
                updated_at: r.get(13),
            })
            .collect();

        // Recurring events — fetch all and expand
        let recurring_rows = sqlx::query(
            "SELECT id, profile_id, title, description, start_time, end_time, location, scope, category, all_day, rrule, reminder_minutes_before, created_at, updated_at
             FROM events WHERE profile_id = ?1 AND rrule IS NOT NULL
             ORDER BY start_time ASC",
        )
        .bind(profile_id)
        .fetch_all(pool)
        .await?;

        for row in recurring_rows {
            let base_event = Event {
                id: row.get(0),
                profile_id: row.get(1),
                title: row.get(2),
                description: row.get(3),
                start_time: row.get(4),
                end_time: row.get(5),
                location: row.get(6),
                scope: row.get(7),
                category: row.get(8),
                all_day: row.get(9),
                rrule: row.get(10),
                reminder_minutes_before: row.get(11),
                created_at: row.get(12),
                updated_at: row.get(13),
            };

            if let Some(ref rrule_str) = base_event.rrule {
                let instances = expand_recurring(
                    &base_event.start_time,
                    &base_event.end_time,
                    rrule_str,
                    start,
                    end,
                );

                for (inst_start, inst_end) in instances {
                    let mut inst_event = base_event.clone();
                    inst_event.start_time = inst_start;
                    inst_event.end_time = inst_end;
                    events.push(inst_event);
                }
            }
        }

        // Sort merged list by start_time
        events.sort_by(|a, b| a.start_time.cmp(&b.start_time));

        Ok(events)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update(
        pool: &SqlitePool,
        id: &str,
        title: Option<&str>,
        description: Option<&str>,
        location: Option<&str>,
        category: Option<&str>,
        all_day: Option<bool>,
        rrule: Option<&str>,
        reminder_minutes_before: Option<i32>,
        start_time: Option<&str>,
        end_time: Option<&str>,
    ) -> Result<bool, sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE events SET title = COALESCE(?1, title), description = COALESCE(?2, description), location = COALESCE(?3, location), category = COALESCE(?4, category), all_day = COALESCE(?5, all_day), rrule = COALESCE(?6, rrule), reminder_minutes_before = COALESCE(?7, reminder_minutes_before), start_time = COALESCE(?10, start_time), end_time = COALESCE(?11, end_time), updated_at = ?8 WHERE id = ?9",
        )
        .bind(title)
        .bind(description)
        .bind(location)
        .bind(category)
        .bind(all_day)
        .bind(rrule)
        .bind(reminder_minutes_before)
        .bind(&now)
        .bind(id)
        .bind(start_time)
        .bind(end_time)
        .execute(pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn find_free_slots(
        pool: &SqlitePool,
        profile_id: &str,
        date: &str,
        duration_min: i64,
    ) -> Result<Vec<(String, String)>, sqlx::Error> {
        let day_start = format!("{}T00:00:00Z", date);
        let day_end = format!("{}T23:59:59Z", date);
        let events = Self::list_by_date_range(pool, profile_id, &day_start, &day_end).await?;

        fn parse_dt(s: &str) -> Option<DateTime<FixedOffset>> {
            DateTime::parse_from_rfc3339(s).ok()
        }

        let mut slots = Vec::new();
        let mut cursor = day_start.clone();

        for event in &events {
            if cursor < event.start_time {
                let gap_start = parse_dt(&cursor);
                let gap_end = parse_dt(&event.start_time);
                if let (Some(start), Some(end)) = (gap_start, gap_end) {
                    let gap_minutes = (end - start).num_minutes();
                    if gap_minutes >= duration_min {
                        slots.push((cursor.clone(), event.start_time.clone()));
                    }
                }
            }
            if cursor < event.end_time {
                cursor = event.end_time.clone();
            }
        }

        if let (Some(start), Some(end)) = (parse_dt(&cursor), parse_dt(&day_end)) {
            if (end - start).num_minutes() >= duration_min {
                slots.push((cursor, day_end));
            }
        }

        Ok(slots)
    }
}

/// Parse RRULE strings and generate recurring event instances.
///
/// Supported formats:
/// - `FREQ=DAILY`
/// - `FREQ=WEEKLY;BYDAY=MO,WE,FR`
/// - `FREQ=MONTHLY`
///
/// Returns up to 365 expansions to prevent infinite loops.
/// Only instances whose start_time falls within `range_start..range_end` are returned.
pub fn expand_recurring(
    base_start: &str,
    base_end: &str,
    rrule: &str,
    range_start: &str,
    range_end: &str,
) -> Vec<(String, String)> {
    let max_expansions: usize = 365;

    let Ok(base_start_dt) = DateTime::parse_from_rfc3339(base_start) else {
        return vec![];
    };
    let Ok(base_end_dt) = DateTime::parse_from_rfc3339(base_end) else {
        return vec![];
    };
    let Ok(range_start_dt) = DateTime::parse_from_rfc3339(range_start) else {
        return vec![];
    };
    let Ok(range_end_dt) = DateTime::parse_from_rfc3339(range_end) else {
        return vec![];
    };

    let duration = base_end_dt - base_start_dt;
    let offset = *base_start_dt.offset();

    // Parse RRULE parts
    let parts: std::collections::HashMap<String, String> = rrule
        .split(';')
        .filter_map(|part| {
            let mut kv = part.splitn(2, '=');
            let key = kv.next()?.to_string();
            let value = kv.next()?.to_string();
            Some((key, value))
        })
        .collect();

    let freq = parts.get("FREQ").map(|s| s.as_str()).unwrap_or("DAILY");
    let byday = parts.get("BYDAY").map(|s| s.as_str()).unwrap_or("");

    // Parse BYDAY into weekdays
    let weekdays: Vec<Weekday> = if byday.is_empty() {
        vec![]
    } else {
        byday
            .split(',')
            .filter_map(|d| match d {
                "MO" => Some(Weekday::Mon),
                "TU" => Some(Weekday::Tue),
                "WE" => Some(Weekday::Wed),
                "TH" => Some(Weekday::Thu),
                "FR" => Some(Weekday::Fri),
                "SA" => Some(Weekday::Sat),
                "SU" => Some(Weekday::Sun),
                _ => None,
            })
            .collect()
    };

    let mut results: Vec<(String, String)> = Vec::new();

    match freq {
        "DAILY" => {
            let mut current = base_start_dt;
            let mut count = 0;
            while current < range_end_dt && count < max_expansions {
                if current >= range_start_dt {
                    let end = current + duration;
                    results.push((fmt_rfc3339(current), fmt_rfc3339(end)));
                }
                current += Duration::days(1);
                count += 1;
            }
        }

        "WEEKLY" => {
            if weekdays.is_empty() {
                // Simple weekly: every 7 days from base
                let mut current = base_start_dt;
                let mut count = 0;
                while current < range_end_dt && count < max_expansions {
                    if current >= range_start_dt {
                        let end = current + duration;
                        results.push((fmt_rfc3339(current), fmt_rfc3339(end)));
                    }
                    current += Duration::days(7);
                    count += 1;
                }
            } else {
                // BYDAY: iterate day by day, matching specific weekdays
                let mut current = base_start_dt;
                let mut count = 0;
                while current < range_end_dt && count < max_expansions {
                    if current >= range_start_dt && weekdays.contains(&current.weekday()) {
                        let end = current + duration;
                        results.push((fmt_rfc3339(current), fmt_rfc3339(end)));
                    }
                    current += Duration::days(1);
                    count += 1;
                }
            }
        }

        "MONTHLY" => {
            let base_naive = base_start_dt.naive_utc();
            let range_start_naive = range_start_dt.naive_utc();
            let range_end_naive = range_end_dt.naive_utc();

            let mut current_naive = base_naive;
            let mut count = 0;
            while current_naive < range_end_naive && count < max_expansions {
                if current_naive >= range_start_naive {
                    let end_naive = current_naive + duration;
                    let start_formatted = fmt_rfc3339(
                        DateTime::<FixedOffset>::from_naive_utc_and_offset(current_naive, offset),
                    );
                    let end_formatted = fmt_rfc3339(
                        DateTime::<FixedOffset>::from_naive_utc_and_offset(end_naive, offset),
                    );
                    results.push((start_formatted, end_formatted));
                }
                // Advance one month
                if let Some(next) = add_month_naive(current_naive) {
                    current_naive = next;
                } else {
                    break;
                }
                count += 1;
            }
        }

        _ => {}
    }

    results
}

/// Helper: advance a NaiveDateTime by one calendar month, clamping day to
/// the target month's max.
fn add_month_naive(dt: NaiveDateTime) -> Option<NaiveDateTime> {
    let mut year = dt.year();
    let mut month = dt.month() + 1;
    if month > 12 {
        month = 1;
        year += 1;
    }
    let day = dt.day().min(days_in_month(year, month));
    NaiveDate::from_ymd_opt(year, month, day)
        .map(|d| d.and_hms_opt(dt.hour(), dt.minute(), dt.second()).unwrap())
}

/// Number of days in a given month/year.
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        _ => 28,
    }
}

/// Format a DateTime as RFC 3339, always using "Z" for UTC offset.
fn fmt_rfc3339(dt: DateTime<FixedOffset>) -> String {
    // chrono's to_rfc3339() uses "+00:00" for UTC; format manually with "Z"
    format!("{}Z", dt.format("%Y-%m-%dT%H:%M:%S"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    async fn setup() -> Result<SqlitePool, sqlx::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await?;
        sqlx::migrate::Migrator::new(std::path::Path::new("migrations"))
            .await
            .unwrap()
            .run(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO profiles (id, name, preferences) VALUES ('profile-1', 'Test', '{}')",
        )
        .execute(&pool)
        .await?;
        Ok(pool)
    }

    fn make_event(id: &str) -> Event {
        Event {
            id: id.into(),
            profile_id: "profile-1".into(),
            title: "Test Event".into(),
            description: None,
            start_time: "2026-09-24T10:00:00Z".into(),
            end_time: "2026-09-24T11:00:00Z".into(),
            location: None,
            scope: "shared".into(),
            category: "default".into(),
            all_day: false,
            rrule: None,
            reminder_minutes_before: None,
            created_at: "2026-09-23T00:00:00Z".into(),
            updated_at: "2026-09-23T00:00:00Z".into(),
        }
    }

    // ── create / find_by_id ────────────────────────────────────────────────

    #[tokio::test]
    async fn test_create_event() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        let event = Event {
            id: "evt-1".into(),
            profile_id: "profile-1".into(),
            title: "Reunión".into(),
            description: None,
            start_time: "2026-09-24T10:00:00Z".into(),
            end_time: "2026-09-24T11:00:00Z".into(),
            location: None,
            scope: "shared".into(),
            category: "meeting".into(),
            all_day: false,
            rrule: None,
            reminder_minutes_before: Some(15),
            created_at: "2026-09-23T00:00:00Z".into(),
            updated_at: "2026-09-23T00:00:00Z".into(),
        };
        EventsRepo::create(&pool, &event).await.unwrap();
        let found = EventsRepo::find_by_id(&pool, "evt-1").await?.unwrap();
        assert_eq!(found.title, "Reunión");
        assert_eq!(found.scope, "shared");
        assert_eq!(found.category, "meeting");
        assert_eq!(found.all_day, false);
        assert_eq!(found.rrule, None);
        assert_eq!(found.reminder_minutes_before, Some(15));

        Ok(())
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        let found = EventsRepo::find_by_id(&pool, "nonexistent").await.unwrap();
        assert!(found.is_none());

        Ok(())
    }

    // ── delete ─────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_delete_existing() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        let event = make_event("evt-del-1");
        EventsRepo::create(&pool, &event).await.unwrap();

        // Confirm it exists
        assert!(EventsRepo::find_by_id(&pool, "evt-del-1").await?.is_some());

        // Delete
        EventsRepo::delete(&pool, "evt-del-1").await.unwrap();

        // Confirm it's gone
        assert!(EventsRepo::find_by_id(&pool, "evt-del-1").await?.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_nonexistent() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        // Should return false — no event with that id
        assert!(!EventsRepo::delete(&pool, "does-not-exist").await?);
        Ok(())
    }

    // ── list_by_date_range (non-recurring) ─────────────────────────────────

    #[tokio::test]
    async fn test_list_by_date_range() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        let event = Event {
            id: "evt-2".into(),
            profile_id: "profile-1".into(),
            title: "Cita".into(),
            description: None,
            start_time: "2026-09-24T15:00:00Z".into(),
            end_time: "2026-09-24T16:00:00Z".into(),
            location: None,
            scope: "personal".into(),
            category: "default".into(),
            all_day: false,
            rrule: None,
            reminder_minutes_before: None,
            created_at: "2026-09-23T00:00:00Z".into(),
            updated_at: "2026-09-23T00:00:00Z".into(),
        };
        EventsRepo::create(&pool, &event).await.unwrap();
        let events = EventsRepo::list_by_date_range(
            &pool,
            "profile-1",
            "2026-09-24T00:00:00Z",
            "2026-09-24T23:59:59Z",
        )
        .await?;
        assert_eq!(events.len(), 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_list_by_date_range_outside() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        let event = Event {
            id: "evt-3".into(),
            profile_id: "profile-1".into(),
            title: "Fuera de rango".into(),
            description: None,
            start_time: "2026-09-25T10:00:00Z".into(),
            end_time: "2026-09-25T11:00:00Z".into(),
            location: None,
            scope: "shared".into(),
            category: "default".into(),
            all_day: false,
            rrule: None,
            reminder_minutes_before: None,
            created_at: "2026-09-23T00:00:00Z".into(),
            updated_at: "2026-09-23T00:00:00Z".into(),
        };
        EventsRepo::create(&pool, &event).await.unwrap();
        let events = EventsRepo::list_by_date_range(
            &pool,
            "profile-1",
            "2026-09-24T00:00:00Z",
            "2026-09-24T23:59:59Z",
        )
        .await?;
        assert!(events.is_empty());

        Ok(())
    }

    // ── list_by_date_range with recurring events ──────────────────────────

    #[tokio::test]
    async fn test_list_by_date_range_with_recurring() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;

        // Non-recurring event in range
        let non_rec = Event {
            id: "evt-normal".into(),
            profile_id: "profile-1".into(),
            title: "Normal".into(),
            description: None,
            start_time: "2026-09-24T10:00:00Z".into(),
            end_time: "2026-09-24T11:00:00Z".into(),
            location: None,
            scope: "shared".into(),
            category: "default".into(),
            all_day: false,
            rrule: None,
            reminder_minutes_before: None,
            created_at: "2026-09-23T00:00:00Z".into(),
            updated_at: "2026-09-23T00:00:00Z".into(),
        };
        EventsRepo::create(&pool, &non_rec).await.unwrap();

        // Recurring daily event starting 2026-09-20
        let recurring = Event {
            id: "evt-daily".into(),
            profile_id: "profile-1".into(),
            title: "Daily Standup".into(),
            description: None,
            start_time: "2026-09-20T09:00:00Z".into(),
            end_time: "2026-09-20T09:30:00Z".into(),
            location: None,
            scope: "shared".into(),
            category: "default".into(),
            all_day: false,
            rrule: Some("FREQ=DAILY".into()),
            reminder_minutes_before: None,
            created_at: "2026-09-19T00:00:00Z".into(),
            updated_at: "2026-09-19T00:00:00Z".into(),
        };
        EventsRepo::create(&pool, &recurring).await.unwrap();

        // Query range: 2026-09-24
        let events = EventsRepo::list_by_date_range(
            &pool,
            "profile-1",
            "2026-09-24T00:00:00Z",
            "2026-09-24T23:59:59Z",
        )
        .await?;

        // Should include Normal (10:00-11:00) + Daily Standup (09:00-09:30)
        assert_eq!(events.len(), 2, "should have normal + recurring expansion");
        // Sorted: Daily Standup at 09:00, then Normal at 10:00
        assert_eq!(events[0].title, "Daily Standup");
        assert_eq!(events[1].title, "Normal");

        Ok(())
    }

    // ── list_by_category ───────────────────────────────────────────────────

    #[tokio::test]
    async fn test_list_by_category() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;

        let e1 = Event {
            id: "evt-cat-1".into(),
            profile_id: "profile-1".into(),
            title: "Meeting".into(),
            description: None,
            start_time: "2026-09-24T10:00:00Z".into(),
            end_time: "2026-09-24T11:00:00Z".into(),
            location: None,
            scope: "shared".into(),
            category: "meeting".into(),
            all_day: false,
            rrule: None,
            reminder_minutes_before: None,
            created_at: "2026-09-23T00:00:00Z".into(),
            updated_at: "2026-09-23T00:00:00Z".into(),
        };
        let e2 = Event {
            id: "evt-cat-2".into(),
            profile_id: "profile-1".into(),
            title: "Birthday".into(),
            description: None,
            start_time: "2026-09-25T10:00:00Z".into(),
            end_time: "2026-09-25T11:00:00Z".into(),
            location: None,
            scope: "personal".into(),
            category: "birthday".into(),
            all_day: true,
            rrule: Some("FREQ=YEARLY".into()),
            reminder_minutes_before: Some(1440),
            created_at: "2026-09-23T00:00:00Z".into(),
            updated_at: "2026-09-23T00:00:00Z".into(),
        };
        EventsRepo::create(&pool, &e1).await.unwrap();
        EventsRepo::create(&pool, &e2).await.unwrap();

        let meetings = EventsRepo::list_by_category(&pool, "profile-1", "meeting").await?;
        assert_eq!(meetings.len(), 1);
        assert_eq!(meetings[0].title, "Meeting");

        let birthdays = EventsRepo::list_by_category(&pool, "profile-1", "birthday").await?;
        assert_eq!(birthdays.len(), 1);
        assert_eq!(birthdays[0].title, "Birthday");
        assert!(birthdays[0].all_day);
        assert_eq!(birthdays[0].rrule.as_deref(), Some("FREQ=YEARLY"));
        assert_eq!(birthdays[0].reminder_minutes_before, Some(1440));

        let empty = EventsRepo::list_by_category(&pool, "profile-1", "nonexistent").await?;
        assert!(empty.is_empty());

        Ok(())
    }

    // ── update ─────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_update_event() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        let event = Event {
            id: "evt-4".into(),
            profile_id: "profile-1".into(),
            title: "Original".into(),
            description: None,
            start_time: "2026-09-24T10:00:00Z".into(),
            end_time: "2026-09-24T11:00:00Z".into(),
            location: None,
            scope: "shared".into(),
            category: "default".into(),
            all_day: false,
            rrule: None,
            reminder_minutes_before: None,
            created_at: "2026-09-23T00:00:00Z".into(),
            updated_at: "2026-09-23T00:00:00Z".into(),
        };
        EventsRepo::create(&pool, &event).await.unwrap();
        EventsRepo::update(
            &pool,
            "evt-4",
            Some("Actualizado"),
            None,
            Some("Oficina"),
            None,
            None,
            None,
            None,
            Some("2026-09-25T14:00:00Z"),
            Some("2026-09-25T15:00:00Z"),
        )
        .await?;
        let found = EventsRepo::find_by_id(&pool, "evt-4").await?.unwrap();
        assert_eq!(found.title, "Actualizado");
        assert_eq!(found.location.unwrap(), "Oficina");
        assert_eq!(found.start_time, "2026-09-25T14:00:00Z");
        assert_eq!(found.end_time, "2026-09-25T15:00:00Z");

        Ok(())
    }

    // ── find_free_slots ────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_find_free_slots() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        let event = Event {
            id: "evt-5".into(),
            profile_id: "profile-1".into(),
            title: "Ocupado".into(),
            description: None,
            start_time: "2026-09-24T10:00:00Z".into(),
            end_time: "2026-09-24T11:00:00Z".into(),
            location: None,
            scope: "shared".into(),
            category: "default".into(),
            all_day: false,
            rrule: None,
            reminder_minutes_before: None,
            created_at: "".into(),
            updated_at: "".into(),
        };
        EventsRepo::create(&pool, &event).await.unwrap();
        let slots = EventsRepo::find_free_slots(&pool, "profile-1", "2026-09-24", 30).await?;
        assert!(!slots.is_empty());
        // Should have a slot before 10:00 and after 11:00
        assert!(slots.len() >= 2);

        Ok(())
    }

    // ── expand_recurring unit tests ────────────────────────────────────────

    #[test]
    fn test_expand_recurring_daily() {
        let results = expand_recurring(
            "2026-09-24T10:00:00Z",
            "2026-09-24T11:00:00Z",
            "FREQ=DAILY",
            "2026-09-25T00:00:00Z",
            "2026-09-28T00:00:00Z",
        );

        // Should have instances on Sep 25, 26, 27 (3 days within range)
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].0, "2026-09-25T10:00:00Z");
        assert_eq!(results[0].1, "2026-09-25T11:00:00Z");
        assert_eq!(results[1].0, "2026-09-26T10:00:00Z");
        assert_eq!(results[2].0, "2026-09-27T10:00:00Z");
    }

    #[test]
    fn test_expand_recurring_weekly() {
        // Base: Monday 2026-09-21 (a Monday)
        // Recur Mon/Wed/Fri
        let results = expand_recurring(
            "2026-09-21T09:00:00Z", // Monday
            "2026-09-21T10:00:00Z",
            "FREQ=WEEKLY;BYDAY=MO,WE,FR",
            "2026-09-21T00:00:00Z",
            "2026-09-28T00:00:00Z",
        );

        // Week of Sep 21: Mon 21, Wed 23, Fri 25 = 3 instances
        // Week of Sep 28 starts Sep 28 (Monday) which is at the boundary
        // Range is [Sep 21, Sep 28), so Sep 28 is excluded
        assert_eq!(results.len(), 3, "expected Mon 21, Wed 23, Fri 25");
        assert_eq!(results[0].0, "2026-09-21T09:00:00Z");
        assert_eq!(results[1].0, "2026-09-23T09:00:00Z");
        assert_eq!(results[2].0, "2026-09-25T09:00:00Z");
    }

    #[test]
    fn test_expand_recurring_monthly() {
        let results = expand_recurring(
            "2026-01-15T10:00:00Z",
            "2026-01-15T11:00:00Z",
            "FREQ=MONTHLY",
            "2026-03-01T00:00:00Z",
            "2026-06-01T00:00:00Z",
        );

        // Mar 15, Apr 15, May 15 = 3 instances
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].0, "2026-03-15T10:00:00Z");
        assert_eq!(results[1].0, "2026-04-15T10:00:00Z");
        assert_eq!(results[2].0, "2026-05-15T10:00:00Z");
    }

    #[test]
    fn test_expand_recurring_outside_range() {
        let results = expand_recurring(
            "2026-01-01T10:00:00Z",
            "2026-01-01T11:00:00Z",
            "FREQ=DAILY",
            "2026-06-01T00:00:00Z",
            "2026-06-05T00:00:00Z",
        );

        // Jan 1 daily, but range starts Jun 1 — should expand from Jan 1
        // forward until we reach the range, producing up to 365 results
        // The first result should be Jun 1
        assert!(!results.is_empty());
        assert_eq!(results[0].0, "2026-06-01T10:00:00Z");
        assert_eq!(results.last().unwrap().0, "2026-06-04T10:00:00Z");
    }

    #[test]
    fn test_expand_recurring_invalid_rrule() {
        let results = expand_recurring(
            "2026-09-24T10:00:00Z",
            "2026-09-24T11:00:00Z",
            "FREQ=YEARLY",
            "2026-09-24T00:00:00Z",
            "2026-09-25T00:00:00Z",
        );
        // YEARLY is not supported — returns empty
        assert!(results.is_empty());
    }

    #[test]
    fn test_expand_recurring_max_365() {
        let results = expand_recurring(
            "2026-01-01T00:00:00Z",
            "2026-01-01T01:00:00Z",
            "FREQ=DAILY",
            "2026-01-01T00:00:00Z",
            "2030-01-01T00:00:00Z",
        );
        // Should cap at 365
        assert_eq!(results.len(), 365);
    }

    #[test]
    fn test_expand_recurring_bad_dates() {
        // Invalid datetime strings → empty
        let results = expand_recurring("bad-date", "bad-date", "FREQ=DAILY", "bad", "bad");
        assert!(results.is_empty());
    }
}
