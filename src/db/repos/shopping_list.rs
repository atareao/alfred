use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    fn row_to_item(row: &rusqlite::Row) -> SqlResult<ShoppingItem> {
        Ok(ShoppingItem {
            id: row.get(0)?,
            profile_id: row.get(1)?,
            item: row.get(2)?,
            quantity: row.get(3)?,
            category: row.get(4)?,
            checked: row.get(5)?,
            created_at: row.get(6)?,
        })
    }

    pub fn list(
        conn: &Connection,
        profile_id: &str,
        category: Option<&str>,
    ) -> SqlResult<Vec<ShoppingItem>> {
        let mut sql = String::from(
            "SELECT id, profile_id, item, quantity, category, checked, created_at
             FROM shopping_list WHERE profile_id = ?1",
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> =
            vec![Box::new(profile_id.to_string())];

        if let Some(cat) = category {
            param_values.push(Box::new(cat.to_string()));
            sql.push_str(&format!(" AND category = ?{}", param_values.len()));
        }

        sql.push_str(" ORDER BY category, item");

        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), Self::row_to_item)?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    pub fn add(
        conn: &Connection,
        profile_id: &str,
        item: &str,
        quantity: Option<&str>,
        category: Option<&str>,
    ) -> SqlResult<ShoppingItem> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO shopping_list (id, profile_id, item, quantity, category, checked, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)",
            params![id, profile_id, item, quantity, category, now],
        )?;
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
    pub fn check_off(conn: &Connection, profile_id: &str, item: &str) -> SqlResult<bool> {
        let affected = conn.execute(
            "UPDATE shopping_list SET checked = 1
             WHERE profile_id = ?1 AND LOWER(item) = LOWER(?2)",
            params![profile_id, item],
        )?;
        Ok(affected > 0)
    }

    /// Deletes all checked items for the given profile.
    /// Returns the number of rows deleted.
    pub fn delete_checked(conn: &Connection, profile_id: &str) -> SqlResult<usize> {
        let affected = conn.execute(
            "DELETE FROM shopping_list WHERE checked = 1 AND profile_id = ?1",
            params![profile_id],
        )?;
        Ok(affected)
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
    fn test_add_and_list() {
        let conn = setup();
        ShoppingListRepo::add(
            &conn,
            "profile-1",
            "Leche",
            Some("1 litro"),
            Some("Lácteos"),
        )
        .unwrap();
        ShoppingListRepo::add(&conn, "profile-1", "Pan", None, Some("Panadería")).unwrap();

        let items = ShoppingListRepo::list(&conn, "profile-1", None).unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_list_filter_by_category() {
        let conn = setup();
        ShoppingListRepo::add(&conn, "profile-1", "Leche", None, Some("Lácteos")).unwrap();
        ShoppingListRepo::add(&conn, "profile-1", "Manzanas", None, Some("Frutas")).unwrap();

        let lacteos = ShoppingListRepo::list(&conn, "profile-1", Some("Lácteos")).unwrap();
        assert_eq!(lacteos.len(), 1);
        assert_eq!(lacteos[0].item, "Leche");
    }

    #[test]
    fn test_list_ordering() {
        let conn = setup();
        ShoppingListRepo::add(&conn, "profile-1", "Zanahoria", None, Some("Verduras")).unwrap();
        ShoppingListRepo::add(&conn, "profile-1", "Acelga", None, Some("Verduras")).unwrap();
        ShoppingListRepo::add(&conn, "profile-1", "Leche", None, Some("Lácteos")).unwrap();

        let items = ShoppingListRepo::list(&conn, "profile-1", None).unwrap();
        // Ordered by category, item: Lácteos/Leche, Verduras/Acelga, Verduras/Zanahoria
        assert_eq!(items[0].item, "Leche");
        assert_eq!(items[1].item, "Acelga");
        assert_eq!(items[2].item, "Zanahoria");
    }

    #[test]
    fn test_check_off() {
        let conn = setup();
        ShoppingListRepo::add(&conn, "profile-1", "Leche", None, None).unwrap();

        assert!(ShoppingListRepo::check_off(&conn, "profile-1", "Leche").unwrap());

        let items = ShoppingListRepo::list(&conn, "profile-1", None).unwrap();
        assert!(items[0].checked);
    }

    #[test]
    fn test_check_off_case_insensitive() {
        let conn = setup();
        ShoppingListRepo::add(&conn, "profile-1", "Leche", None, None).unwrap();

        assert!(ShoppingListRepo::check_off(&conn, "profile-1", "leche").unwrap());

        let items = ShoppingListRepo::list(&conn, "profile-1", None).unwrap();
        assert!(items[0].checked);
    }

    #[test]
    fn test_check_off_not_found() {
        let conn = setup();
        assert!(!ShoppingListRepo::check_off(&conn, "profile-1", "Inexistente").unwrap());
    }

    #[test]
    fn test_delete_checked() {
        let conn = setup();
        ShoppingListRepo::add(&conn, "profile-1", "Leche", None, None).unwrap();
        ShoppingListRepo::add(&conn, "profile-1", "Pan", None, None).unwrap();
        ShoppingListRepo::check_off(&conn, "profile-1", "Leche").unwrap();

        let deleted = ShoppingListRepo::delete_checked(&conn, "profile-1").unwrap();
        assert_eq!(deleted, 1);

        let items = ShoppingListRepo::list(&conn, "profile-1", None).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].item, "Pan");
    }

    #[test]
    fn test_delete_checked_none() {
        let conn = setup();
        ShoppingListRepo::add(&conn, "profile-1", "Leche", None, None).unwrap();
        let deleted = ShoppingListRepo::delete_checked(&conn, "profile-1").unwrap();
        assert_eq!(deleted, 0);
    }

    #[test]
    fn test_list_empty() {
        let conn = setup();
        let items = ShoppingListRepo::list(&conn, "profile-1", None).unwrap();
        assert!(items.is_empty());
    }
}
