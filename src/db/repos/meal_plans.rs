use chrono::Datelike;
use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MealPlan {
    pub id: String,
    pub profile_id: String,
    pub week_start: String,
    pub meals: Value,
    pub created_at: String,
}

pub struct MealPlansRepo;

impl MealPlansRepo {
    /// Returns the Monday (ISO) of the current week.
    fn current_week_monday() -> String {
        let today = chrono::Utc::now().date_naive();
        let days_from_monday = today.weekday().num_days_from_monday();
        let monday = today - chrono::Duration::days(days_from_monday.into());
        monday.format("%Y-%m-%d").to_string()
    }

    fn row_to_meal_plan(row: &rusqlite::Row) -> SqlResult<MealPlan> {
        let meals_str: String = row.get(3)?;
        Ok(MealPlan {
            id: row.get(0)?,
            profile_id: row.get(1)?,
            week_start: row.get(2)?,
            meals: serde_json::from_str(&meals_str).unwrap_or_default(),
            created_at: row.get(4)?,
        })
    }

    pub fn get_current(conn: &Connection, profile_id: &str) -> SqlResult<Option<MealPlan>> {
        let week_start = Self::current_week_monday();
        let mut stmt = conn.prepare(
            "SELECT id, profile_id, week_start, meals, created_at
             FROM meal_plans
             WHERE profile_id = ?1 AND week_start = ?2",
        )?;
        let mut rows = stmt.query_map(params![profile_id, week_start], Self::row_to_meal_plan)?;
        match rows.next() {
            Some(Ok(plan)) => Ok(Some(plan)),
            _ => Ok(None),
        }
    }

    fn find_by_profile_and_week(
        conn: &Connection,
        profile_id: &str,
        week_start: &str,
    ) -> SqlResult<Option<MealPlan>> {
        let mut stmt = conn.prepare(
            "SELECT id, profile_id, week_start, meals, created_at
             FROM meal_plans
             WHERE profile_id = ?1 AND week_start = ?2",
        )?;
        let mut rows = stmt.query_map(params![profile_id, week_start], Self::row_to_meal_plan)?;
        match rows.next() {
            Some(Ok(plan)) => Ok(Some(plan)),
            _ => Ok(None),
        }
    }

    pub fn upsert(
        conn: &Connection,
        profile_id: &str,
        week_start: &str,
        meals: &Value,
    ) -> SqlResult<MealPlan> {
        let now = chrono::Utc::now().to_rfc3339();
        let meals_str = meals.to_string();

        if let Some(existing) = Self::find_by_profile_and_week(conn, profile_id, week_start)? {
            conn.execute(
                "UPDATE meal_plans SET meals = ?1, created_at = ?2 WHERE id = ?3",
                params![meals_str, now, existing.id],
            )?;
            Ok(MealPlan {
                id: existing.id,
                profile_id: profile_id.to_string(),
                week_start: week_start.to_string(),
                meals: meals.clone(),
                created_at: now,
            })
        } else {
            let id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO meal_plans (id, profile_id, week_start, meals, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, profile_id, week_start, meals_str, now],
            )?;
            Ok(MealPlan {
                id,
                profile_id: profile_id.to_string(),
                week_start: week_start.to_string(),
                meals: meals.clone(),
                created_at: now,
            })
        }
    }

    pub fn delete(conn: &Connection, id: &str) -> SqlResult<bool> {
        let affected = conn.execute("DELETE FROM meal_plans WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }
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

    #[test]
    fn test_create_and_get_current() {
        let conn = setup();
        let meals: Value = serde_json::json!({
            "monday": {"breakfast": "avena"},
            "tuesday": {"lunch": "ensalada"}
        });
        let week_start = MealPlansRepo::current_week_monday();

        let plan = MealPlansRepo::upsert(&conn, "profile-1", &week_start, &meals).unwrap();
        assert_eq!(plan.profile_id, "profile-1");
        assert_eq!(plan.meals["monday"]["breakfast"], "avena");

        let current = MealPlansRepo::get_current(&conn, "profile-1")
            .unwrap()
            .unwrap();
        assert_eq!(current.id, plan.id);
        assert_eq!(current.week_start, week_start);
    }

    #[test]
    fn test_upsert_updates_existing() {
        let conn = setup();
        let week_start = MealPlansRepo::current_week_monday();
        let meals1: Value = serde_json::json!({"day": "original"});
        let plan1 = MealPlansRepo::upsert(&conn, "profile-1", &week_start, &meals1).unwrap();

        let meals2: Value = serde_json::json!({"day": "updated"});
        let plan2 = MealPlansRepo::upsert(&conn, "profile-1", &week_start, &meals2).unwrap();

        assert_eq!(plan1.id, plan2.id);
        assert_eq!(plan2.meals["day"], "updated");

        let current = MealPlansRepo::get_current(&conn, "profile-1")
            .unwrap()
            .unwrap();
        assert_eq!(current.meals["day"], "updated");
    }

    #[test]
    fn test_get_current_not_found() {
        let conn = setup();
        let result = MealPlansRepo::get_current(&conn, "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_delete() {
        let conn = setup();
        let week_start = MealPlansRepo::current_week_monday();
        let meals: Value = serde_json::json!({});
        let plan = MealPlansRepo::upsert(&conn, "profile-1", &week_start, &meals).unwrap();

        assert!(MealPlansRepo::delete(&conn, &plan.id).unwrap());
        let current = MealPlansRepo::get_current(&conn, "profile-1").unwrap();
        assert!(current.is_none());
    }

    #[test]
    fn test_delete_not_found() {
        let conn = setup();
        assert!(!MealPlansRepo::delete(&conn, "nonexistent").unwrap());
    }
}
