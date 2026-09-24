use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Habit {
    pub id: String,
    pub profile_id: String,
    pub name: String,
    pub frequency: String,
    pub target: Option<u32>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HabitLog {
    pub habit_id: String,
    pub date: String,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HabitStreak {
    pub habit_id: String,
    pub name: String,
    pub current_streak: u32,
    pub longest_streak: u32,
    pub total_count: u32,
}

pub struct HabitsRepo;

impl HabitsRepo {
    fn row_to_habit(row: &rusqlite::Row) -> SqlResult<Habit> {
        Ok(Habit {
            id: row.get(0)?,
            profile_id: row.get(1)?,
            name: row.get(2)?,
            frequency: row.get(3)?,
            target: row.get(4)?,
            created_at: row.get(5)?,
        })
    }

    pub fn create(
        conn: &Connection,
        profile_id: &str,
        name: &str,
        frequency: &str,
        target: Option<u32>,
    ) -> SqlResult<Habit> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO habits (id, profile_id, name, frequency, target, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, profile_id, name, frequency, target, now],
        )?;
        Ok(Habit {
            id,
            profile_id: profile_id.to_string(),
            name: name.to_string(),
            frequency: frequency.to_string(),
            target,
            created_at: now,
        })
    }

    pub fn list(conn: &Connection, profile_id: &str) -> SqlResult<Vec<Habit>> {
        let mut stmt = conn.prepare(
            "SELECT id, profile_id, name, frequency, target, created_at
             FROM habits WHERE profile_id = ?1
             ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![profile_id], Self::row_to_habit)?;
        let mut habits = Vec::new();
        for row in rows {
            habits.push(row?);
        }
        Ok(habits)
    }

    pub fn find_by_id(conn: &Connection, id: &str) -> SqlResult<Option<Habit>> {
        let mut stmt = conn.prepare(
            "SELECT id, profile_id, name, frequency, target, created_at
             FROM habits WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], Self::row_to_habit)?;
        match rows.next() {
            Some(Ok(habit)) => Ok(Some(habit)),
            _ => Ok(None),
        }
    }

    /// Log a habit completion for a given date.
    /// Uses INSERT OR REPLACE so logging twice on the same day keeps the completed=1 state.
    pub fn log(conn: &Connection, habit_id: &str, date: &str) -> SqlResult<()> {
        conn.execute(
            "INSERT OR REPLACE INTO habit_logs (habit_id, date, completed)
             VALUES (?1, ?2, 1)",
            params![habit_id, date],
        )?;
        Ok(())
    }

    /// Compute streaks (current and longest) plus total count for every habit of a profile.
    ///
    /// Current streak counts consecutive days backwards from today where the habit
    /// has a log entry. Longest streak is the longest historical run of consecutive
    /// logged days.
    pub fn get_streaks(conn: &Connection, profile_id: &str) -> SqlResult<Vec<HabitStreak>> {
        let habits = Self::list(conn, profile_id)?;
        let today = chrono::Utc::now().date_naive();

        let mut streaks = Vec::new();
        for habit in &habits {
            // Fetch all completed log dates for this habit, ordered DESC
            let mut stmt = conn.prepare(
                "SELECT date FROM habit_logs
                 WHERE habit_id = ?1 AND completed = 1
                 ORDER BY date DESC",
            )?;
            let raw_dates: Vec<String> = stmt
                .query_map(params![habit.id], |row| row.get(0))?
                .filter_map(|r| r.ok())
                .collect();

            let total_count = raw_dates.len() as u32;

            eprintln!(
                "DEBUG get_streaks: habit={}, raw_dates={:?}, today={:?}",
                habit.id, raw_dates, today
            );

            // Parse dates
            let dates: Vec<chrono::NaiveDate> = raw_dates
                .iter()
                .filter_map(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
                .collect();

            let current_streak = compute_current_streak(&dates, today);
            let longest_streak = compute_longest_streak(&dates);

            streaks.push(HabitStreak {
                habit_id: habit.id.clone(),
                name: habit.name.clone(),
                current_streak,
                longest_streak,
                total_count,
            });
        }

        Ok(streaks)
    }

    /// Get stats for a single habit over a given period.
    ///
    /// Returns `(completed, expected)` where:
    /// - `completed` = number of days the habit was logged in the period
    /// - `expected` = target number of completions (days for daily, weeks for weekly)
    pub fn get_stats(conn: &Connection, habit_id: &str, period: &str) -> SqlResult<(u32, u32)> {
        let days = match period {
            "week" => 7,
            "month" => 30,
            "year" => 365,
            _ => 7,
        };

        let completed: u32 = conn.query_row(
            "SELECT COUNT(*) FROM habit_logs
             WHERE habit_id = ?1 AND completed = 1
               AND date >= date('now', '-' || ?2 || ' days')",
            params![habit_id, days.to_string()],
            |row| row.get(0),
        )?;

        // Determine expected count from the habit's frequency
        let frequency: String = conn.query_row(
            "SELECT frequency FROM habits WHERE id = ?1",
            params![habit_id],
            |row| row.get(0),
        )?;

        let expected = match frequency.as_str() {
            "daily" => days,
            "weekly" => (days as f64 / 7.0).ceil() as u32,
            _ => days,
        };

        Ok((completed, expected))
    }
}

// ── Helper: streak computation ──────────────────────────────────────────────

/// Given a sorted (descending) list of logged dates, compute the current streak
/// of consecutive days ending at (and including) `today`.
fn compute_current_streak(dates: &[chrono::NaiveDate], today: chrono::NaiveDate) -> u32 {
    if dates.is_empty() {
        return 0;
    }
    // If today isn't logged, streak is 0
    if dates[0] != today {
        return 0;
    }

    let mut streak = 1u32;
    let mut expected = today.pred_opt().unwrap_or(today);
    for d in &dates[1..] {
        if *d == expected {
            streak += 1;
            expected = match expected.pred_opt() {
                Some(prev) => prev,
                None => break,
            };
        } else if *d < expected {
            break;
        }
    }
    streak
}

/// Given a sorted (descending) list of logged dates, compute the longest
/// consecutive run. Returns 0 for empty input.
fn compute_longest_streak(dates: &[chrono::NaiveDate]) -> u32 {
    if dates.is_empty() {
        return 0;
    }
    // Work with ascending order for easier run detection
    let ascending: Vec<chrono::NaiveDate> = dates.iter().copied().rev().collect();

    let mut longest = 1u32;
    let mut current_run = 1u32;

    for pair in ascending.windows(2) {
        let earlier = pair[0];
        let later = pair[1];
        // consecutive days: later == earlier.succ_opt()
        if later == earlier.succ_opt().unwrap_or(earlier) {
            current_run += 1;
        } else {
            longest = longest.max(current_run);
            current_run = 1;
        }
    }
    longest.max(current_run)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::run_migrations;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn.execute(
            "INSERT INTO profiles (id, name, preferences) VALUES ('profile-1', 'Test', '{}')",
            [],
        )
        .unwrap();
        conn
    }

    fn create_daily_habit(conn: &Connection, name: &str) -> Habit {
        HabitsRepo::create(conn, "profile-1", name, "daily", None).unwrap()
    }

    // ── Habit CRUD ──────────────────────────────────────────────────────────

    #[test]
    fn test_create_habit() {
        let conn = setup();
        let habit = create_daily_habit(&conn, "Ejercicio");
        assert_eq!(habit.name, "Ejercicio");
        assert_eq!(habit.frequency, "daily");

        let found = HabitsRepo::find_by_id(&conn, &habit.id).unwrap().unwrap();
        assert_eq!(found.name, "Ejercicio");
    }

    #[test]
    fn test_list_habits() {
        let conn = setup();
        create_daily_habit(&conn, "Leer");
        create_daily_habit(&conn, "Meditar");

        let habits = HabitsRepo::list(&conn, "profile-1").unwrap();
        assert_eq!(habits.len(), 2);
    }

    #[test]
    fn test_find_by_id_not_found() {
        let conn = setup();
        let result = HabitsRepo::find_by_id(&conn, "nonexistent").unwrap();
        assert!(result.is_none());
    }

    // ── Habit Logging ───────────────────────────────────────────────────────

    #[test]
    fn test_log_habit() {
        let conn = setup();
        let habit = create_daily_habit(&conn, "Agua");

        HabitsRepo::log(&conn, &habit.id, "2026-09-21").unwrap();
        HabitsRepo::log(&conn, &habit.id, "2026-09-22").unwrap();

        // Repeated log on same day should not fail
        HabitsRepo::log(&conn, &habit.id, "2026-09-22").unwrap();
    }

    // ── Streaks ─────────────────────────────────────────────────────────────

    #[test]
    fn test_streaks_empty() {
        let conn = setup();
        let _habit = create_daily_habit(&conn, "Vacío");
        let streaks = HabitsRepo::get_streaks(&conn, "profile-1").unwrap();
        assert_eq!(streaks.len(), 1);
        assert_eq!(streaks[0].current_streak, 0);
        assert_eq!(streaks[0].longest_streak, 0);
        assert_eq!(streaks[0].total_count, 0);
    }

    #[test]
    fn test_current_streak_active() {
        let today = chrono::Utc::now().date_naive();
        let conn = setup();
        let habit = create_daily_habit(&conn, "Correr");

        // Log today and the two previous days
        HabitsRepo::log(&conn, &habit.id, &today.to_string()).unwrap();
        HabitsRepo::log(&conn, &habit.id, &today.pred_opt().unwrap().to_string()).unwrap();
        HabitsRepo::log(
            &conn,
            &habit.id,
            &today.pred_opt().unwrap().pred_opt().unwrap().to_string(),
        )
        .unwrap();

        let streaks = HabitsRepo::get_streaks(&conn, "profile-1").unwrap();
        assert_eq!(streaks[0].current_streak, 3);
        assert_eq!(streaks[0].longest_streak, 3);
        assert_eq!(streaks[0].total_count, 3);
    }

    #[test]
    fn test_current_streak_no_today() {
        let today = chrono::Utc::now().date_naive();
        let conn = setup();
        let habit = create_daily_habit(&conn, "Escribir");

        // Log yesterday but not today
        HabitsRepo::log(&conn, &habit.id, &today.pred_opt().unwrap().to_string()).unwrap();

        let streaks = HabitsRepo::get_streaks(&conn, "profile-1").unwrap();
        assert_eq!(streaks[0].current_streak, 0);
        assert_eq!(streaks[0].longest_streak, 1);
        assert_eq!(streaks[0].total_count, 1);
    }

    #[test]
    fn test_longest_streak_gt_current() {
        let today = chrono::Utc::now().date_naive();
        let conn = setup();
        let habit = create_daily_habit(&conn, "Meditar");

        // Log: 3-day streak a week ago, then just today
        let week_ago = today - chrono::Duration::days(7);
        for i in 0..3 {
            let d = week_ago + chrono::Duration::days(i);
            HabitsRepo::log(&conn, &habit.id, &d.to_string()).unwrap();
        }
        HabitsRepo::log(&conn, &habit.id, &today.to_string()).unwrap();

        let streaks = HabitsRepo::get_streaks(&conn, "profile-1").unwrap();
        assert_eq!(streaks[0].current_streak, 1);
        assert_eq!(streaks[0].longest_streak, 3);
        assert_eq!(streaks[0].total_count, 4);
    }

    // ── Stats ───────────────────────────────────────────────────────────────

    #[test]
    fn test_get_stats_weekly() {
        let conn = setup();
        let habit =
            HabitsRepo::create(&conn, "profile-1", "Jardinería", "weekly", Some(1)).unwrap();
        HabitsRepo::log(&conn, &habit.id, "2026-09-21").unwrap();

        let (completed, expected) = HabitsRepo::get_stats(&conn, &habit.id, "month").unwrap();
        assert_eq!(completed, 1);
        // 30 days ≈ 5 weeks
        assert_eq!(expected, 5);
    }

    #[test]
    fn test_get_stats_daily() {
        let conn = setup();
        let habit = create_daily_habit(&conn, "Agua");

        let today = chrono::Utc::now().date_naive();
        for i in 0..5 {
            let d = today - chrono::Duration::days(i);
            HabitsRepo::log(&conn, &habit.id, &d.to_string()).unwrap();
        }

        let (completed, expected) = HabitsRepo::get_stats(&conn, &habit.id, "week").unwrap();
        assert_eq!(completed, 5);
        assert_eq!(expected, 7);
    }

    // ── Helper unit tests ───────────────────────────────────────────────────

    #[test]
    fn test_compute_current_streak_simple() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 23).unwrap();
        let dates = vec![
            today,
            today.pred_opt().unwrap(),
            today.pred_opt().unwrap().pred_opt().unwrap(),
        ];
        assert_eq!(compute_current_streak(&dates, today), 3);
    }

    #[test]
    fn test_compute_current_streak_no_today() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 23).unwrap();
        let yesterday = today.pred_opt().unwrap();
        let dates = vec![yesterday];
        assert_eq!(compute_current_streak(&dates, today), 0);
    }

    #[test]
    fn test_compute_longest_streak_empty() {
        assert_eq!(compute_longest_streak(&[]), 0);
    }
}
