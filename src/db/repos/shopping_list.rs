use chrono::Utc;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ShoppingItem {
    pub id: String,
    pub profile_id: String,
    pub item: String,
    pub quantity: Option<String>,
    pub category: Option<String>,
    pub checked: bool,
    pub created_at: String,
}

pub struct ShoppingListRepo;

impl ShoppingListRepo {
    pub async fn list(
        pool: &SqlitePool,
        profile_id: &str,
        category: Option<&str>,
    ) -> Result<Vec<ShoppingItem>, sqlx::Error> {
        let mut sql = String::from(
            "SELECT id, profile_id, item, quantity, category, checked, created_at
             FROM shopping_list WHERE profile_id = ?1",
        );

        if category.is_some() {
            sql.push_str(" AND category = ?2");
        }

        sql.push_str(" ORDER BY category, item");

        let rows = if let Some(cat) = category {
            sqlx::query(&sql)
                .bind(profile_id)
                .bind(cat)
                .fetch_all(pool)
                .await?
        } else {
            sqlx::query(&sql).bind(profile_id).fetch_all(pool).await?
        };

        let items: Vec<ShoppingItem> = rows
            .iter()
            .map(|row| ShoppingItem {
                id: row.get(0),
                profile_id: row.get(1),
                item: row.get(2),
                quantity: row.get(3),
                category: row.get(4),
                checked: row.get::<bool, _>(5),
                created_at: row.get(6),
            })
            .collect();

        Ok(items)
    }

    pub async fn add(
        pool: &SqlitePool,
        profile_id: &str,
        item: &str,
        quantity: Option<&str>,
        category: Option<&str>,
    ) -> Result<ShoppingItem, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO shopping_list (id, profile_id, item, quantity, category, checked, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)",
        )
        .bind(&id)
        .bind(profile_id)
        .bind(item)
        .bind(quantity)
        .bind(category)
        .bind(&now)
        .execute(pool)
        .await?;

        Ok(ShoppingItem {
            id,
            profile_id: profile_id.to_string(),
            item: item.to_string(),
            quantity: quantity.map(|s| s.to_string()),
            category: category.map(|s| s.to_string()),
            checked: false,
            created_at: now,
        })
    }

    /// Marks an item as checked (case-insensitive matching on item name).
    /// Returns true if a row was updated.
    pub async fn check_off(
        pool: &SqlitePool,
        profile_id: &str,
        item: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE shopping_list SET checked = 1
             WHERE profile_id = ?1 AND LOWER(item) = LOWER(?2)",
        )
        .bind(profile_id)
        .bind(item)
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Deletes all checked items for the given profile.
    /// Returns the number of rows deleted.
    pub async fn delete_checked(pool: &SqlitePool, profile_id: &str) -> Result<usize, sqlx::Error> {
        let result = sqlx::query("DELETE FROM shopping_list WHERE checked = 1 AND profile_id = ?1")
            .bind(profile_id)
            .execute(pool)
            .await?;

        Ok(result.rows_affected() as usize)
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

        // Insert a profile row so FK constraints are satisfied
        sqlx::query("INSERT INTO profiles (id, name, preferences) VALUES (?1, 'Test', '{}')")
            .bind("profile-1")
            .execute(&pool)
            .await?;

        Ok(pool)
    }

    #[tokio::test]
    async fn test_add_and_list() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;

        ShoppingListRepo::add(
            &pool,
            "profile-1",
            "Leche",
            Some("1 litro"),
            Some("Lácteos"),
        )
        .await?;
        ShoppingListRepo::add(&pool, "profile-1", "Pan", None, Some("Panadería")).await?;

        let items = ShoppingListRepo::list(&pool, "profile-1", None).await?;
        assert_eq!(items.len(), 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_list_filter_by_category() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;

        ShoppingListRepo::add(&pool, "profile-1", "Leche", None, Some("Lácteos")).await?;
        ShoppingListRepo::add(&pool, "profile-1", "Manzanas", None, Some("Frutas")).await?;

        let lacteos = ShoppingListRepo::list(&pool, "profile-1", Some("Lácteos")).await?;
        assert_eq!(lacteos.len(), 1);
        assert_eq!(lacteos[0].item, "Leche");

        Ok(())
    }

    #[tokio::test]
    async fn test_list_ordering() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;

        ShoppingListRepo::add(&pool, "profile-1", "Zanahoria", None, Some("Verduras")).await?;
        ShoppingListRepo::add(&pool, "profile-1", "Acelga", None, Some("Verduras")).await?;
        ShoppingListRepo::add(&pool, "profile-1", "Leche", None, Some("Lácteos")).await?;

        let items = ShoppingListRepo::list(&pool, "profile-1", None).await?;
        // Ordered by category, item: Lácteos/Leche, Verduras/Acelga, Verduras/Zanahoria
        assert_eq!(items[0].item, "Leche");
        assert_eq!(items[1].item, "Acelga");
        assert_eq!(items[2].item, "Zanahoria");

        Ok(())
    }

    #[tokio::test]
    async fn test_check_off() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;

        ShoppingListRepo::add(&pool, "profile-1", "Leche", None, None).await?;

        assert!(ShoppingListRepo::check_off(&pool, "profile-1", "Leche").await?);

        let items = ShoppingListRepo::list(&pool, "profile-1", None).await?;
        assert!(items[0].checked);

        Ok(())
    }

    #[tokio::test]
    async fn test_check_off_case_insensitive() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;

        ShoppingListRepo::add(&pool, "profile-1", "Leche", None, None).await?;

        assert!(ShoppingListRepo::check_off(&pool, "profile-1", "leche").await?);

        let items = ShoppingListRepo::list(&pool, "profile-1", None).await?;
        assert!(items[0].checked);

        Ok(())
    }

    #[tokio::test]
    async fn test_check_off_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;

        assert!(!ShoppingListRepo::check_off(&pool, "profile-1", "Inexistente").await?);

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_checked() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;

        ShoppingListRepo::add(&pool, "profile-1", "Leche", None, None).await?;
        ShoppingListRepo::add(&pool, "profile-1", "Pan", None, None).await?;
        ShoppingListRepo::check_off(&pool, "profile-1", "Leche").await?;

        let deleted = ShoppingListRepo::delete_checked(&pool, "profile-1").await?;
        assert_eq!(deleted, 1);

        let items = ShoppingListRepo::list(&pool, "profile-1", None).await?;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].item, "Pan");

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_checked_none() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;

        ShoppingListRepo::add(&pool, "profile-1", "Leche", None, None).await?;
        let deleted = ShoppingListRepo::delete_checked(&pool, "profile-1").await?;
        assert_eq!(deleted, 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_list_empty() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;

        let items = ShoppingListRepo::list(&pool, "profile-1", None).await?;
        assert!(items.is_empty());

        Ok(())
    }
}
