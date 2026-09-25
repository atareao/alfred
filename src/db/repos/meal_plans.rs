use chrono::Datelike;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Row, SqlitePool};

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

    fn row_to_meal_plan(row: &sqlx::sqlite::SqliteRow) -> MealPlan {
        let meals_str: String = row.get("meals");
        MealPlan {
            id: row.get("id"),
            profile_id: row.get("profile_id"),
            week_start: row.get("week_start"),
            meals: serde_json::from_str(&meals_str).unwrap_or_default(),
            created_at: row.get("created_at"),
        }
    }

    pub async fn get_current(
        pool: &SqlitePool,
        profile_id: &str,
    ) -> Result<Option<MealPlan>, sqlx::Error> {
        let week_start = Self::current_week_monday();
        let row = sqlx::query(
            "SELECT id, profile_id, week_start, meals, created_at
             FROM meal_plans
             WHERE profile_id = ?1 AND week_start = ?2",
        )
        .bind(profile_id)
        .bind(&week_start)
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|r| Self::row_to_meal_plan(&r)))
    }

    async fn find_by_profile_and_week(
        pool: &SqlitePool,
        profile_id: &str,
        week_start: &str,
    ) -> Result<Option<MealPlan>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, profile_id, week_start, meals, created_at
             FROM meal_plans
             WHERE profile_id = ?1 AND week_start = ?2",
        )
        .bind(profile_id)
        .bind(week_start)
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|r| Self::row_to_meal_plan(&r)))
    }

    pub async fn upsert(
        pool: &SqlitePool,
        profile_id: &str,
        week_start: &str,
        meals: &Value,
    ) -> Result<MealPlan, sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        let meals_str = meals.to_string();

        if let Some(existing) = Self::find_by_profile_and_week(pool, profile_id, week_start).await?
        {
            sqlx::query("UPDATE meal_plans SET meals = ?1, created_at = ?2 WHERE id = ?3")
                .bind(&meals_str)
                .bind(&now)
                .bind(&existing.id)
                .execute(pool)
                .await?;
            Ok(MealPlan {
                id: existing.id,
                profile_id: profile_id.to_string(),
                week_start: week_start.to_string(),
                meals: meals.clone(),
                created_at: now,
            })
        } else {
            let id = uuid::Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO meal_plans (id, profile_id, week_start, meals, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )
            .bind(&id)
            .bind(profile_id)
            .bind(week_start)
            .bind(&meals_str)
            .bind(&now)
            .execute(pool)
            .await?;
            Ok(MealPlan {
                id,
                profile_id: profile_id.to_string(),
                week_start: week_start.to_string(),
                meals: meals.clone(),
                created_at: now,
            })
        }
    }

    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
        let affected = sqlx::query("DELETE FROM meal_plans WHERE id = ?1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(affected.rows_affected() > 0)
    }
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

    #[tokio::test]
    async fn test_create_and_get_current() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        let meals: Value = serde_json::json!({
            "monday": {"breakfast": "avena"},
            "tuesday": {"lunch": "ensalada"}
        });
        let week_start = MealPlansRepo::current_week_monday();

        let plan = MealPlansRepo::upsert(&pool, "profile-1", &week_start, &meals).await?;
        assert_eq!(plan.profile_id, "profile-1");
        assert_eq!(plan.meals["monday"]["breakfast"], "avena");

        let current = MealPlansRepo::get_current(&pool, "profile-1")
            .await?
            .unwrap();
        assert_eq!(current.id, plan.id);
        assert_eq!(current.week_start, week_start);

        Ok(())
    }

    #[tokio::test]
    async fn test_upsert_updates_existing() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        let week_start = MealPlansRepo::current_week_monday();
        let meals1: Value = serde_json::json!({"day": "original"});
        let plan1 = MealPlansRepo::upsert(&pool, "profile-1", &week_start, &meals1).await?;

        let meals2: Value = serde_json::json!({"day": "updated"});
        let plan2 = MealPlansRepo::upsert(&pool, "profile-1", &week_start, &meals2).await?;

        assert_eq!(plan1.id, plan2.id);
        assert_eq!(plan2.meals["day"], "updated");

        let current = MealPlansRepo::get_current(&pool, "profile-1")
            .await?
            .unwrap();
        assert_eq!(current.meals["day"], "updated");

        Ok(())
    }

    #[tokio::test]
    async fn test_get_current_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        let result = MealPlansRepo::get_current(&pool, "nonexistent").await?;
        assert!(result.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_delete() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        let week_start = MealPlansRepo::current_week_monday();
        let meals: Value = serde_json::json!({});
        let plan = MealPlansRepo::upsert(&pool, "profile-1", &week_start, &meals).await?;

        assert!(MealPlansRepo::delete(&pool, &plan.id).await.unwrap());
        let current = MealPlansRepo::get_current(&pool, "profile-1").await?;
        assert!(current.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        assert!(!MealPlansRepo::delete(&pool, "nonexistent").await.unwrap());

        Ok(())
    }
}
