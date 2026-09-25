use serde::{Deserialize, Serialize};
use sqlx::{Error, Row, SqlitePool};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: String,
    pub profile_id: String,
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct ContactsRepo;

impl ContactsRepo {
    pub async fn create(pool: &SqlitePool, contact: &Contact) -> Result<(), Error> {
        sqlx::query(
            "INSERT INTO contacts (id, profile_id, name, phone, email, notes, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(&contact.id)
        .bind(&contact.profile_id)
        .bind(&contact.name)
        .bind(&contact.phone)
        .bind(&contact.email)
        .bind(&contact.notes)
        .bind(&contact.created_at)
        .bind(&contact.updated_at)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Contact>, Error> {
        let row = sqlx::query(
            "SELECT id, profile_id, name, phone, email, notes, created_at, updated_at
             FROM contacts WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        match row {
            Some(row) => Ok(Some(Contact {
                id: row.get(0),
                profile_id: row.get(1),
                name: row.get(2),
                phone: row.get(3),
                email: row.get(4),
                notes: row.get(5),
                created_at: row.get(6),
                updated_at: row.get(7),
            })),
            None => Ok(None),
        }
    }

    /// Search contacts by name, phone, or email using LIKE.
    pub async fn search(
        pool: &SqlitePool,
        profile_id: &str,
        query: &str,
    ) -> Result<Vec<Contact>, Error> {
        let pattern = format!("%{}%", query);
        let rows = sqlx::query(
            "SELECT id, profile_id, name, phone, email, notes, created_at, updated_at
             FROM contacts
             WHERE profile_id = ? AND (name LIKE ? OR phone LIKE ? OR email LIKE ?)
             ORDER BY name ASC",
        )
        .bind(profile_id)
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .fetch_all(pool)
        .await?;

        let contacts: Vec<Contact> = rows
            .iter()
            .map(|row| Contact {
                id: row.get(0),
                profile_id: row.get(1),
                name: row.get(2),
                phone: row.get(3),
                email: row.get(4),
                notes: row.get(5),
                created_at: row.get(6),
                updated_at: row.get(7),
            })
            .collect();
        Ok(contacts)
    }

    pub async fn update(
        pool: &SqlitePool,
        id: &str,
        name: Option<&str>,
        phone: Option<&str>,
        email: Option<&str>,
        notes: Option<&str>,
    ) -> Result<(), Error> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE contacts SET
                name = COALESCE(?, name),
                phone = COALESCE(?, phone),
                email = COALESCE(?, email),
                notes = COALESCE(?, notes),
                updated_at = ?
             WHERE id = ?",
        )
        .bind(name)
        .bind(phone)
        .bind(email)
        .bind(notes)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), Error> {
        sqlx::query("DELETE FROM contacts WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    /// Creates an in-memory SQLite pool with the minimum tables needed for
    /// contacts tests (profiles + contacts).
    async fn setup() -> Result<SqlitePool, sqlx::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;

        // Create the profiles and contacts tables (schema subset)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS profiles (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                preferences TEXT NOT NULL DEFAULT '{}'
            )",
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS contacts (
                id TEXT PRIMARY KEY,
                profile_id TEXT NOT NULL REFERENCES profiles(id),
                name TEXT NOT NULL,
                phone TEXT,
                email TEXT,
                notes TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            "INSERT INTO profiles (id, name, preferences) VALUES ('profile-1', 'Test', '{}')",
        )
        .execute(&pool)
        .await?;

        Ok(pool)
    }

    fn sample_contact(id: &str, name: &str, phone: Option<&str>, email: Option<&str>) -> Contact {
        Contact {
            id: id.into(),
            profile_id: "profile-1".into(),
            name: name.into(),
            phone: phone.map(|s| s.into()),
            email: email.map(|s| s.into()),
            notes: None,
            created_at: "2026-09-23T00:00:00Z".into(),
            updated_at: "2026-09-23T00:00:00Z".into(),
        }
    }

    #[tokio::test]
    async fn test_create_contact() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        let contact = sample_contact(
            "c1",
            "Juan Pérez",
            Some("+34 600 000 000"),
            Some("juan@example.com"),
        );
        ContactsRepo::create(&pool, &contact).await?;
        let found = ContactsRepo::find_by_id(&pool, "c1").await?.unwrap();
        assert_eq!(found.name, "Juan Pérez");
        assert_eq!(found.email.unwrap(), "juan@example.com");
        Ok(())
    }

    #[tokio::test]
    async fn test_search_by_name() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        ContactsRepo::create(&pool, &sample_contact("c2", "María García", None, None)).await?;
        ContactsRepo::create(&pool, &sample_contact("c3", "Carlos López", None, None)).await?;
        ContactsRepo::create(&pool, &sample_contact("c4", "Ana Martínez", None, None)).await?;

        let results = ContactsRepo::search(&pool, "profile-1", "María").await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "María García");

        let results = ContactsRepo::search(&pool, "profile-1", "ez").await?;
        assert_eq!(results.len(), 2); // López & Martínez both contain "ez"
        Ok(())
    }

    #[tokio::test]
    async fn test_search_by_phone() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        ContactsRepo::create(
            &pool,
            &sample_contact("c5", "Pedro", Some("+34 611 111 111"), None),
        )
        .await?;
        ContactsRepo::create(
            &pool,
            &sample_contact("c6", "Laura", Some("+34 622 222 222"), None),
        )
        .await?;

        let results = ContactsRepo::search(&pool, "profile-1", "611").await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Pedro");
        Ok(())
    }

    #[tokio::test]
    async fn test_search_by_email() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        ContactsRepo::create(
            &pool,
            &sample_contact("c7", "Luis", None, Some("luis@work.com")),
        )
        .await?;
        ContactsRepo::create(
            &pool,
            &sample_contact("c8", "Elena", None, Some("elena@personal.com")),
        )
        .await?;

        let results = ContactsRepo::search(&pool, "profile-1", "work").await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Luis");
        Ok(())
    }

    #[tokio::test]
    async fn test_update_contact() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        ContactsRepo::create(&pool, &sample_contact("c9", "Nombre Original", None, None)).await?;
        ContactsRepo::update(
            &pool,
            "c9",
            Some("Nombre Nuevo"),
            Some("+34 600 000 001"),
            None,
            Some("Nota importante"),
        )
        .await?;
        let found = ContactsRepo::find_by_id(&pool, "c9").await?.unwrap();
        assert_eq!(found.name, "Nombre Nuevo");
        assert_eq!(found.phone.unwrap(), "+34 600 000 001");
        assert_eq!(found.notes.unwrap(), "Nota importante");
        Ok(())
    }

    #[tokio::test]
    async fn test_search_no_results() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        ContactsRepo::create(&pool, &sample_contact("c10", "Solo yo", None, None)).await?;
        let results = ContactsRepo::search(&pool, "profile-1", "ZzzNadie").await?;
        assert!(results.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_contact() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        ContactsRepo::create(&pool, &sample_contact("c11", "Para borrar", None, None)).await?;
        ContactsRepo::delete(&pool, "c11").await?;
        let found = ContactsRepo::find_by_id(&pool, "c11").await?;
        assert!(found.is_none());
        Ok(())
    }
}
