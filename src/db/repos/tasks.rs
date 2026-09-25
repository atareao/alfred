use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub profile_id: String,
    pub content: String,
    pub status: String,
    pub priority: String,
    pub project: Option<String>,
    pub due_date: Option<String>,
    pub scope: String,
    pub created_at: String,
    pub updated_at: String,
}

pub struct TasksRepo;

impl TasksRepo {
    pub async fn create(pool: &SqlitePool, task: &Task) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO tasks (id, profile_id, content, status, priority, project, due_date, scope, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )
        .bind(&task.id)
        .bind(&task.profile_id)
        .bind(&task.content)
        .bind(&task.status)
        .bind(&task.priority)
        .bind(&task.project)
        .bind(&task.due_date)
        .bind(&task.scope)
        .bind(&task.created_at)
        .bind(&task.updated_at)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Task>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, profile_id, content, status, priority, project, due_date, scope, created_at, updated_at
             FROM tasks WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        match row {
            Some(r) => Ok(Some(Task {
                id: r.get("id"),
                profile_id: r.get("profile_id"),
                content: r.get("content"),
                status: r.get("status"),
                priority: r.get("priority"),
                project: r.get("project"),
                due_date: r.get("due_date"),
                scope: r.get("scope"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            })),
            None => Ok(None),
        }
    }

    /// List tasks with optional filters. All filter parameters are optional.
    #[allow(clippy::too_many_arguments)]
    pub async fn list(
        pool: &SqlitePool,
        profile_id: &str,
        status: Option<&str>,
        priority: Option<&str>,
        project: Option<&str>,
        scope: Option<&str>,
    ) -> Result<Vec<Task>, sqlx::Error> {
        let mut sql = String::from(
            "SELECT id, profile_id, content, status, priority, project, due_date, scope, created_at, updated_at
             FROM tasks WHERE profile_id = ?1",
        );
        let mut param_idx = 2u8;

        if status.is_some() {
            sql.push_str(&format!(" AND status = ?{param_idx}"));
            param_idx += 1;
        }
        if priority.is_some() {
            sql.push_str(&format!(" AND priority = ?{param_idx}"));
            param_idx += 1;
        }
        if project.is_some() {
            sql.push_str(&format!(" AND project = ?{param_idx}"));
            param_idx += 1;
        }
        if scope.is_some() {
            sql.push_str(&format!(" AND scope = ?{param_idx}"));
        }

        sql.push_str(" ORDER BY created_at DESC");

        let mut query = sqlx::query(&sql).bind(profile_id);
        if let Some(s) = status {
            query = query.bind(s);
        }
        if let Some(p) = priority {
            query = query.bind(p);
        }
        if let Some(p) = project {
            query = query.bind(p);
        }
        if let Some(s) = scope {
            query = query.bind(s);
        }

        let rows = query.fetch_all(pool).await?;

        let tasks: Vec<Task> = rows
            .iter()
            .map(|r| Task {
                id: r.get("id"),
                profile_id: r.get("profile_id"),
                content: r.get("content"),
                status: r.get("status"),
                priority: r.get("priority"),
                project: r.get("project"),
                due_date: r.get("due_date"),
                scope: r.get("scope"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            })
            .collect();

        Ok(tasks)
    }

    pub async fn update(
        pool: &SqlitePool,
        id: &str,
        content: Option<&str>,
        priority: Option<&str>,
        project: Option<&str>,
        due_date: Option<&str>,
        scope: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE tasks SET
                content = COALESCE(?1, content),
                priority = COALESCE(?2, priority),
                project = COALESCE(?3, project),
                due_date = COALESCE(?4, due_date),
                scope = COALESCE(?5, scope),
                updated_at = ?6
             WHERE id = ?7",
        )
        .bind(content)
        .bind(priority)
        .bind(project)
        .bind(due_date)
        .bind(scope)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn complete(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query("UPDATE tasks SET status = 'completed', updated_at = ?1 WHERE id = ?2")
            .bind(&now)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn cancel(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query("UPDATE tasks SET status = 'cancelled', updated_at = ?1 WHERE id = ?2")
            .bind(&now)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM tasks WHERE id = ?1")
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

    async fn setup() -> Result<SqlitePool, sqlx::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;

        // Create profiles table
        sqlx::query(
            "CREATE TABLE profiles (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                preferences TEXT NOT NULL DEFAULT '{}'
            )",
        )
        .execute(&pool)
        .await?;

        // Create tasks table
        sqlx::query(
            "CREATE TABLE tasks (
                id TEXT PRIMARY KEY,
                profile_id TEXT NOT NULL REFERENCES profiles(id),
                content TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                priority TEXT NOT NULL DEFAULT 'medium',
                project TEXT,
                due_date TEXT,
                scope TEXT NOT NULL DEFAULT 'shared',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await?;

        // Insert test profile
        sqlx::query(
            "INSERT INTO profiles (id, name, preferences) VALUES ('profile-1', 'Test', '{}')",
        )
        .execute(&pool)
        .await?;

        Ok(pool)
    }

    fn sample_task(id: &str, project: Option<&str>, status: &str, priority: &str) -> Task {
        Task {
            id: id.into(),
            profile_id: "profile-1".into(),
            content: format!("Task {}", id),
            status: status.into(),
            priority: priority.into(),
            project: project.map(|s| s.into()),
            due_date: None,
            scope: "shared".into(),
            created_at: "2026-09-23T00:00:00Z".into(),
            updated_at: "2026-09-23T00:00:00Z".into(),
        }
    }

    #[tokio::test]
    async fn test_create_task() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        let task = sample_task("task-1", Some("Proyecto X"), "pending", "high");
        TasksRepo::create(&pool, &task).await.unwrap();
        let found = TasksRepo::find_by_id(&pool, "task-1").await?.unwrap();
        assert_eq!(found.content, "Task task-1");
        assert_eq!(found.priority, "high");

        Ok(())
    }

    #[tokio::test]
    async fn test_list_by_project() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        TasksRepo::create(&pool, &sample_task("t1", Some("Alpha"), "pending", "low")).await?;
        TasksRepo::create(&pool, &sample_task("t2", Some("Beta"), "pending", "high")).await?;
        TasksRepo::create(
            &pool,
            &sample_task("t3", Some("Alpha"), "completed", "medium"),
        )
        .await?;

        let alpha = TasksRepo::list(&pool, "profile-1", None, None, Some("Alpha"), None).await?;
        assert_eq!(alpha.len(), 2);

        let beta = TasksRepo::list(&pool, "profile-1", None, None, Some("Beta"), None).await?;
        assert_eq!(beta.len(), 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_complete_task() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        TasksRepo::create(&pool, &sample_task("t4", None, "pending", "medium")).await?;
        TasksRepo::complete(&pool, "t4").await.unwrap();
        let found = TasksRepo::find_by_id(&pool, "t4").await.unwrap().unwrap();
        assert_eq!(found.status, "completed");

        Ok(())
    }

    #[tokio::test]
    async fn test_cancel_task() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        TasksRepo::create(&pool, &sample_task("t5", None, "pending", "medium")).await?;
        TasksRepo::cancel(&pool, "t5").await.unwrap();
        let found = TasksRepo::find_by_id(&pool, "t5").await.unwrap().unwrap();
        assert_eq!(found.status, "cancelled");

        Ok(())
    }

    #[tokio::test]
    async fn test_update_task() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        TasksRepo::create(&pool, &sample_task("t6", None, "pending", "low")).await?;
        TasksRepo::update(
            &pool,
            "t6",
            Some("Nuevo contenido"),
            Some("high"),
            Some("Proyecto Z"),
            None,
            None,
        )
        .await?;
        let found = TasksRepo::find_by_id(&pool, "t6").await.unwrap().unwrap();
        assert_eq!(found.content, "Nuevo contenido");
        assert_eq!(found.priority, "high");
        assert_eq!(found.project.unwrap(), "Proyecto Z");

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_task() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        TasksRepo::create(&pool, &sample_task("t7", None, "pending", "medium")).await?;
        TasksRepo::delete(&pool, "t7").await.unwrap();
        let found = TasksRepo::find_by_id(&pool, "t7").await.unwrap();
        assert!(found.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_list_by_status_and_priority() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup().await?;
        TasksRepo::create(&pool, &sample_task("t8", None, "pending", "high")).await?;
        TasksRepo::create(&pool, &sample_task("t9", None, "completed", "low")).await?;
        TasksRepo::create(&pool, &sample_task("t10", None, "pending", "low")).await?;

        let pending_high = TasksRepo::list(
            &pool,
            "profile-1",
            Some("pending"),
            Some("high"),
            None,
            None,
        )
        .await?;
        assert_eq!(pending_high.len(), 1);
        assert_eq!(pending_high[0].id, "t8");

        Ok(())
    }
}
