use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};

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
    pub fn create(conn: &Connection, task: &Task) -> SqlResult<()> {
        conn.execute(
            "INSERT INTO tasks (id, profile_id, content, status, priority, project, due_date, scope, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                task.id, task.profile_id, task.content, task.status,
                task.priority, task.project, task.due_date, task.scope,
                task.created_at, task.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn find_by_id(conn: &Connection, id: &str) -> SqlResult<Option<Task>> {
        let mut stmt = conn.prepare(
            "SELECT id, profile_id, content, status, priority, project, due_date, scope, created_at, updated_at
             FROM tasks WHERE id = ?1"
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(Task {
                id: row.get(0)?,
                profile_id: row.get(1)?,
                content: row.get(2)?,
                status: row.get(3)?,
                priority: row.get(4)?,
                project: row.get(5)?,
                due_date: row.get(6)?,
                scope: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;
        match rows.next() {
            Some(Ok(task)) => Ok(Some(task)),
            _ => Ok(None),
        }
    }

    /// List tasks with optional filters. All filter parameters are optional.
    #[allow(clippy::too_many_arguments)]
    pub fn list(
        conn: &Connection,
        profile_id: &str,
        status: Option<&str>,
        priority: Option<&str>,
        project: Option<&str>,
        scope: Option<&str>,
    ) -> SqlResult<Vec<Task>> {
        let mut sql = String::from(
            "SELECT id, profile_id, content, status, priority, project, due_date, scope, created_at, updated_at
             FROM tasks WHERE profile_id = ?1"
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> =
            vec![Box::new(profile_id.to_string())];

        if let Some(s) = status {
            param_values.push(Box::new(s.to_string()));
            sql.push_str(&format!(" AND status = ?{}", param_values.len()));
        }
        if let Some(p) = priority {
            param_values.push(Box::new(p.to_string()));
            sql.push_str(&format!(" AND priority = ?{}", param_values.len()));
        }
        if let Some(p) = project {
            param_values.push(Box::new(p.to_string()));
            sql.push_str(&format!(" AND project = ?{}", param_values.len()));
        }
        if let Some(s) = scope {
            param_values.push(Box::new(s.to_string()));
            sql.push_str(&format!(" AND scope = ?{}", param_values.len()));
        }

        sql.push_str(" ORDER BY created_at DESC");

        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            Ok(Task {
                id: row.get(0)?,
                profile_id: row.get(1)?,
                content: row.get(2)?,
                status: row.get(3)?,
                priority: row.get(4)?,
                project: row.get(5)?,
                due_date: row.get(6)?,
                scope: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;
        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(row?);
        }
        Ok(tasks)
    }

    pub fn update(
        conn: &Connection,
        id: &str,
        content: Option<&str>,
        priority: Option<&str>,
        project: Option<&str>,
        due_date: Option<&str>,
        scope: Option<&str>,
    ) -> SqlResult<()> {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE tasks SET
                content = COALESCE(?1, content),
                priority = COALESCE(?2, priority),
                project = COALESCE(?3, project),
                due_date = COALESCE(?4, due_date),
                scope = COALESCE(?5, scope),
                updated_at = ?6
             WHERE id = ?7",
            params![content, priority, project, due_date, scope, now, id],
        )?;
        Ok(())
    }

    pub fn complete(conn: &Connection, id: &str) -> SqlResult<()> {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE tasks SET status = 'completed', updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    pub fn cancel(conn: &Connection, id: &str) -> SqlResult<()> {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE tasks SET status = 'cancelled', updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    pub fn delete(conn: &Connection, id: &str) -> SqlResult<()> {
        conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
        Ok(())
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

    #[test]
    fn test_create_task() {
        let conn = setup();
        let task = sample_task("task-1", Some("Proyecto X"), "pending", "high");
        TasksRepo::create(&conn, &task).unwrap();
        let found = TasksRepo::find_by_id(&conn, "task-1").unwrap().unwrap();
        assert_eq!(found.content, "Task task-1");
        assert_eq!(found.priority, "high");
    }

    #[test]
    fn test_list_by_project() {
        let conn = setup();
        TasksRepo::create(&conn, &sample_task("t1", Some("Alpha"), "pending", "low")).unwrap();
        TasksRepo::create(&conn, &sample_task("t2", Some("Beta"), "pending", "high")).unwrap();
        TasksRepo::create(
            &conn,
            &sample_task("t3", Some("Alpha"), "completed", "medium"),
        )
        .unwrap();

        let alpha = TasksRepo::list(&conn, "profile-1", None, None, Some("Alpha"), None).unwrap();
        assert_eq!(alpha.len(), 2);

        let beta = TasksRepo::list(&conn, "profile-1", None, None, Some("Beta"), None).unwrap();
        assert_eq!(beta.len(), 1);
    }

    #[test]
    fn test_complete_task() {
        let conn = setup();
        TasksRepo::create(&conn, &sample_task("t4", None, "pending", "medium")).unwrap();
        TasksRepo::complete(&conn, "t4").unwrap();
        let found = TasksRepo::find_by_id(&conn, "t4").unwrap().unwrap();
        assert_eq!(found.status, "completed");
    }

    #[test]
    fn test_cancel_task() {
        let conn = setup();
        TasksRepo::create(&conn, &sample_task("t5", None, "pending", "medium")).unwrap();
        TasksRepo::cancel(&conn, "t5").unwrap();
        let found = TasksRepo::find_by_id(&conn, "t5").unwrap().unwrap();
        assert_eq!(found.status, "cancelled");
    }

    #[test]
    fn test_update_task() {
        let conn = setup();
        TasksRepo::create(&conn, &sample_task("t6", None, "pending", "low")).unwrap();
        TasksRepo::update(
            &conn,
            "t6",
            Some("Nuevo contenido"),
            Some("high"),
            Some("Proyecto Z"),
            None,
            None,
        )
        .unwrap();
        let found = TasksRepo::find_by_id(&conn, "t6").unwrap().unwrap();
        assert_eq!(found.content, "Nuevo contenido");
        assert_eq!(found.priority, "high");
        assert_eq!(found.project.unwrap(), "Proyecto Z");
    }

    #[test]
    fn test_delete_task() {
        let conn = setup();
        TasksRepo::create(&conn, &sample_task("t7", None, "pending", "medium")).unwrap();
        TasksRepo::delete(&conn, "t7").unwrap();
        let found = TasksRepo::find_by_id(&conn, "t7").unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_list_by_status_and_priority() {
        let conn = setup();
        TasksRepo::create(&conn, &sample_task("t8", None, "pending", "high")).unwrap();
        TasksRepo::create(&conn, &sample_task("t9", None, "completed", "low")).unwrap();
        TasksRepo::create(&conn, &sample_task("t10", None, "pending", "low")).unwrap();

        let pending_high = TasksRepo::list(
            &conn,
            "profile-1",
            Some("pending"),
            Some("high"),
            None,
            None,
        )
        .unwrap();
        assert_eq!(pending_high.len(), 1);
        assert_eq!(pending_high[0].id, "t8");
    }
}
