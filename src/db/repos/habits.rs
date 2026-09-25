use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

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
    fn row_to_habit(row: &sqlx::sqlite::SqliteRow) -> Habit {
        Habit {
            id: row.get("id"),
            profile_id: row.get("profile_id"),
            name: row.get("name"),
            frequency: row.get("frequency"),
            target: row.get("target"),
            created_at: row.get("created_at"),
        }
    }

    pub async fn create(
        pool: &SqlitePool,
        profile_id: &str,
        name: &str,
        frequency: &str,
        target: Option<u32>,
    ) -> Result<Habit, sqlx::Error> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO habits (id, profile_id, name, frequency, target, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(&id)
        .bind(profile_id)
        .bind(name)
        .bind(frequency)
        .bind(target)
        .bind(&now)
        .execute(pool)
        .await?;
        Ok(Habit {
            id,
            profile_id: profile_id.to_string(),
            name: name.to_string(),
            frequency: frequency.to_string(),
            target,
            created_at: now,
        })
    }

    pub async fn list(pool: &SqlitePool, profile_id: &str) -> Result<Vec<Habit>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, profile_id, name, frequency, target, created_at
             FROM habits WHERE profile_id = ?1
             ORDER BY created_at DESC",
        )
        .bind(profile_id)
        .fetch_all(pool)
        .await?;

        let habits: Vec<Habit> = rows.iter().map(Self::row_to_habit).collect();
        Ok(habits)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Habit>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, profile_id, name, frequency, target, created_at
             FROM habits WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        match row {
            Some(r) => Ok(Some(Self::row_to_habit(&r))),
            None => Ok(None),
        }
    }

    /// Log a habit completion for a given date.
    /// Uses INSERT OR REPLACE so logging twice on the same day keeps the completed=1 state.
    pub async fn log(pool: &SqlitePool, habit_id: &str, date: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT OR REPLACE INTO habit_logs (habit_id, date, completed)
             VALUES (?1, ?2, 1)",
        )
        .bind(habit_id)
        .bind(date)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Compute streaks (current and longest) plus total count for every habit of a profile.
    ///
    /// Current streak counts consecutive days backwards from today where the habit
    /// has a log entry. Longest streak is the longest historical run of consecutive
    /// logged days.
    pub async fn get_streaks(
        pool: &SqlitePool,
        profile_id: &str,
    ) -> Result<Vec<HabitStreak>, sqlx::Error> {
        let habits = Self::list(pool, profile_id).await?;
        let today = chrono::Utc::now().date_naive();

        let mut streaks = Vec::new();
        for habit in &habits {
            // Fetch all completed log dates for this habit, ordered DESC
            let raw_dates: Vec<String> = sqlx::query_scalar(
                "SELECT date FROM habit_logs
                 WHERE habit_id = ?1 AND completed = 1
                 ORDER BY date DESC",
            )
            .bind(&habit.id)
            .fetch_all(pool)
            .await?;

            let total_count = raw_dates.len() as u32;

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
    pub async fn get_stats(
        pool: &SqlitePool,
        habit_id: &str,
        period: &str,
    ) -> Result<(u32, u32), sqlx::Error> {
        let days = match period {
            "week" => 7,
            "month" => 30,
            "year" => 365,
            _ => 7,
        };

        let completed: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM habit_logs
             WHERE habit_id = ?1 AND completed = 1
               AND date >= date('now', '-' || ?2 || ' days')",
        )
        .bind(habit_id)
        .bind(days.to_string())
        .fetch_one(pool)
        .await?;

        // Determine expected count from the habit's frequency
        let frequency: String = sqlx::query_scalar("SELECT frequency FROM habits WHERE id = ?1")
            .bind(habit_id)
            .fetch_one(pool)
            .await?;

        let expected = match frequency.as_str() {
            "daily" => days,
            "weekly" => (days as f64 / 7.0).ceil() as u32,
            _ => days,
        };

        Ok((completed as u32, expected))
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

    async fn create_daily_habit(pool: &SqlitePool, name: &str) -> Result<Habit, sqlx::Error> {
        let habit = HabitsRepo::create(pool, "profile-1", name, "daily", None).await?;
        Ok(habit)
    }

    // ── Habit CRUD ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_create_habit() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        let habit = create_daily_habit(&pool, "Ejercicio").await?;
        assert_eq!(habit.name, "Ejercicio");
        assert_eq!(habit.frequency, "daily");

        let found = HabitsRepo::find_by_id(&pool, &habit.id).await?.unwrap();
        assert_eq!(found.name, "Ejercicio");

        Ok(())
    }

    #[tokio::test]
    async fn test_list_habits() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        create_daily_habit(&pool, "Leer").await?;
        create_daily_habit(&pool, "Meditar").await?;

        let habits = HabitsRepo::list(&pool, "profile-1").await.unwrap();
        assert_eq!(habits.len(), 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        let result = HabitsRepo::find_by_id(&pool, "nonexistent").await.unwrap();
        assert!(result.is_none());

        Ok(())
    }

    // ── Habit Logging ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_log_habit() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        let habit = create_daily_habit(&pool, "Agua").await?;

        HabitsRepo::log(&pool, &habit.id, "2026-09-21").await?;
        HabitsRepo::log(&pool, &habit.id, "2026-09-22").await?;

        // Repeated log on same day should not fail
        HabitsRepo::log(&pool, &habit.id, "2026-09-22").await?;

        Ok(())
    }

    // ── Streaks ─────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_streaks_empty() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        let _habit = create_daily_habit(&pool, "Vacío").await?;
        let streaks = HabitsRepo::get_streaks(&pool, "profile-1").await.unwrap();
        assert_eq!(streaks.len(), 1);
        assert_eq!(streaks[0].current_streak, 0);
        assert_eq!(streaks[0].longest_streak, 0);
        assert_eq!(streaks[0].total_count, 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_current_streak_active() -> Result<(), Box<dyn std::error::Error>> {
        let today = chrono::Utc::now().date_naive();
        let pool = setup().await?;
        let habit = create_daily_habit(&pool, "Correr").await?;

        // Log today and the two previous days
        HabitsRepo::log(&pool, &habit.id, &today.to_string()).await?;
        HabitsRepo::log(&pool, &habit.id, &today.pred_opt().unwrap().to_string()).await?;
        HabitsRepo::log(
            &pool,
            &habit.id,
            &today.pred_opt().unwrap().pred_opt().unwrap().to_string(),
        )
        .await?;

        let streaks = HabitsRepo::get_streaks(&pool, "profile-1").await.unwrap();
        assert_eq!(streaks[0].current_streak, 3);
        assert_eq!(streaks[0].longest_streak, 3);
        assert_eq!(streaks[0].total_count, 3);

        Ok(())
    }

    #[tokio::test]
    async fn test_current_streak_no_today() -> Result<(), Box<dyn std::error::Error>> {
        let today = chrono::Utc::now().date_naive();
        let pool = setup().await?;
        let habit = create_daily_habit(&pool, "Escribir").await?;

        // Log yesterday but not today
        HabitsRepo::log(&pool, &habit.id, &today.pred_opt().unwrap().to_string()).await?;

        let streaks = HabitsRepo::get_streaks(&pool, "profile-1").await.unwrap();
        assert_eq!(streaks[0].current_streak, 0);
        assert_eq!(streaks[0].longest_streak, 1);
        assert_eq!(streaks[0].total_count, 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_longest_streak_gt_current() -> Result<(), Box<dyn std::error::Error>> {
        let today = chrono::Utc::now().date_naive();
        let pool = setup().await?;
        let habit = create_daily_habit(&pool, "Meditar").await?;

        // Log: 3-day streak a week ago, then just today
        let week_ago = today - chrono::Duration::days(7);
        for i in 0..3 {
            let d = week_ago + chrono::Duration::days(i);
            HabitsRepo::log(&pool, &habit.id, &d.to_string()).await?;
        }
        HabitsRepo::log(&pool, &habit.id, &today.to_string()).await?;

        let streaks = HabitsRepo::get_streaks(&pool, "profile-1").await.unwrap();
        assert_eq!(streaks[0].current_streak, 1);
        assert_eq!(streaks[0].longest_streak, 3);
        assert_eq!(streaks[0].total_count, 4);

        Ok(())
    }

    // ── Stats ───────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_stats_weekly() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        let habit = HabitsRepo::create(&pool, "profile-1", "Jardinería", "weekly", Some(1)).await?;
        HabitsRepo::log(&pool, &habit.id, "2026-09-21").await?;

        let (completed, expected) = HabitsRepo::get_stats(&pool, &habit.id, "month").await?;
        assert_eq!(completed, 1);
        // 30 days ≈ 5 weeks
        assert_eq!(expected, 5);

        Ok(())
    }

    #[tokio::test]
    async fn test_get_stats_daily() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        let habit = create_daily_habit(&pool, "Agua").await?;

        let today = chrono::Utc::now().date_naive();
        for i in 0..5 {
            let d = today - chrono::Duration::days(i);
            HabitsRepo::log(&pool, &habit.id, &d.to_string()).await?;
        }

        let (completed, expected) = HabitsRepo::get_stats(&pool, &habit.id, "week").await?;
        assert_eq!(completed, 5);
        assert_eq!(expected, 7);

        Ok(())
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
