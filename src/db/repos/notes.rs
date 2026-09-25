use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub profile_id: String,
    pub content: String,
    pub category: String,
    pub tags: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct NotesRepo;

impl NotesRepo {
    pub async fn create(pool: &SqlitePool, note: &Note) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO notes (id, profile_id, content, category, tags, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind(&note.id)
        .bind(&note.profile_id)
        .bind(&note.content)
        .bind(&note.category)
        .bind(&note.tags)
        .bind(&note.created_at)
        .bind(&note.updated_at)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Note>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, profile_id, content, category, tags, created_at, updated_at
             FROM notes WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|r| Note {
            id: r.get(0),
            profile_id: r.get(1),
            content: r.get(2),
            category: r.get(3),
            tags: r.get(4),
            created_at: r.get(5),
            updated_at: r.get(6),
        }))
    }

    pub async fn list(
        pool: &SqlitePool,
        profile_id: &str,
        category: Option<&str>,
    ) -> Result<Vec<Note>, sqlx::Error> {
        let rows = if let Some(cat) = category {
            sqlx::query(
                "SELECT id, profile_id, content, category, tags, created_at, updated_at
                 FROM notes WHERE profile_id = ?1 AND category = ?2
                 ORDER BY created_at DESC",
            )
            .bind(profile_id)
            .bind(cat)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query(
                "SELECT id, profile_id, content, category, tags, created_at, updated_at
                 FROM notes WHERE profile_id = ?1
                 ORDER BY created_at DESC",
            )
            .bind(profile_id)
            .fetch_all(pool)
            .await?
        };

        let notes: Vec<Note> = rows
            .iter()
            .map(|row| Note {
                id: row.get(0),
                profile_id: row.get(1),
                content: row.get(2),
                category: row.get(3),
                tags: row.get(4),
                created_at: row.get(5),
                updated_at: row.get(6),
            })
            .collect();

        Ok(notes)
    }

    pub async fn update(
        pool: &SqlitePool,
        id: &str,
        content: Option<&str>,
        category: Option<&str>,
        tags: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE notes SET
                content = COALESCE(?1, content),
                category = COALESCE(?2, category),
                tags = COALESCE(?3, tags),
                updated_at = ?4
             WHERE id = ?5",
        )
        .bind(content)
        .bind(category)
        .bind(tags)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM notes WHERE id = ?1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
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

    fn sample_note(id: &str, content: &str, category: &str, tags: Option<&str>) -> Note {
        Note {
            id: id.into(),
            profile_id: "profile-1".into(),
            content: content.into(),
            category: category.into(),
            tags: tags.map(|s| s.into()),
            created_at: "2026-09-23T00:00:00Z".into(),
            updated_at: "2026-09-23T00:00:00Z".into(),
        }
    }

    #[tokio::test]
    async fn test_create_note() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        let note = sample_note(
            "note-1",
            "Recordar comprar pan",
            "idea",
            Some("compras,casa"),
        );
        NotesRepo::create(&pool, &note).await.unwrap();
        let found = NotesRepo::find_by_id(&pool, "note-1").await?.unwrap();
        assert_eq!(found.content, "Recordar comprar pan");
        assert_eq!(found.category, "idea");
        assert_eq!(found.tags.unwrap(), "compras,casa");

        Ok(())
    }

    #[tokio::test]
    async fn test_list_by_category() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        NotesRepo::create(&pool, &sample_note("n1", "Idea genial", "idea", None)).await?;
        NotesRepo::create(&pool, &sample_note("n2", "Diario del día", "journal", None)).await?;
        NotesRepo::create(&pool, &sample_note("n3", "Dato curioso", "fact", None)).await?;
        NotesRepo::create(&pool, &sample_note("n4", "Otra idea", "idea", None)).await?;

        let ideas = NotesRepo::list(&pool, "profile-1", Some("idea")).await?;
        assert_eq!(ideas.len(), 2);

        let journals = NotesRepo::list(&pool, "profile-1", Some("journal")).await?;
        assert_eq!(journals.len(), 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_list_all() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        NotesRepo::create(&pool, &sample_note("n5", "Nota 1", "idea", None)).await?;
        NotesRepo::create(&pool, &sample_note("n6", "Nota 2", "fact", None)).await?;

        let all = NotesRepo::list(&pool, "profile-1", None).await.unwrap();
        assert_eq!(all.len(), 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_update_note() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        NotesRepo::create(&pool, &sample_note("n7", "Original", "idea", None)).await?;
        NotesRepo::update(
            &pool,
            "n7",
            Some("Actualizado"),
            Some("journal"),
            Some("importante"),
        )
        .await?;
        let found = NotesRepo::find_by_id(&pool, "n7").await.unwrap().unwrap();
        assert_eq!(found.content, "Actualizado");
        assert_eq!(found.category, "journal");
        assert_eq!(found.tags.unwrap(), "importante");

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_note() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        NotesRepo::create(&pool, &sample_note("n8", "Para borrar", "todo", None)).await?;
        NotesRepo::delete(&pool, "n8").await.unwrap();
        let found = NotesRepo::find_by_id(&pool, "n8").await.unwrap();
        assert!(found.is_none());

        Ok(())
    }
}
