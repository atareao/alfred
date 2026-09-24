use axum::{extract::State, Json};
use serde_json::{json, Value};

use crate::AppState;

/// Export all data from the database as a single JSON object.
///
/// Returns a JSON object with one key per table, each containing an array
/// of rows serialized as objects with column-name keys and string/number/null values.
pub async fn export_data(State(state): State<AppState>) -> Json<Value> {
    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(e) => return Json(json!({"error": e.to_string()})),
    };

    Json(export_all_tables(&conn))
}

fn export_all_tables(conn: &rusqlite::Connection) -> Value {
    json!({
        "profiles": export_table(conn, "profiles"),
        "conversations": export_table(conn, "conversations"),
        "messages": export_table(conn, "messages"),
        "events": export_table(conn, "events"),
        "tasks": export_table(conn, "tasks"),
        "notes": export_table(conn, "notes"),
        "contacts": export_table(conn, "contacts"),
        "reminders": export_table(conn, "reminders"),
        "meal_plans": export_table(conn, "meal_plans"),
        "shopping_list": export_table(conn, "shopping_list"),
        "habits": export_table(conn, "habits"),
        "habit_logs": export_table(conn, "habit_logs"),
        "memories": export_table(conn, "memories"),
        "tools": export_table(conn, "tools"),
    })
}

fn export_table(conn: &rusqlite::Connection, table_name: &str) -> Value {
    let query = format!("SELECT * FROM {}", table_name);
    let mut stmt = match conn.prepare(&query) {
        Ok(s) => s,
        Err(_) => return json!([]),
    };

    let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

    let rows = match stmt.query_map([], |row| {
        let mut obj = serde_json::Map::new();
        for (i, name) in column_names.iter().enumerate() {
            let value: Value = match row.get::<_, String>(i) {
                Ok(v) => json!(v),
                Err(_) => match row.get::<_, i64>(i) {
                    Ok(v) => json!(v),
                    Err(_) => Value::Null,
                },
            };
            obj.insert(name.clone(), value);
        }
        Ok(Value::Object(obj))
    }) {
        Ok(r) => r,
        Err(_) => return json!([]),
    };

    let results: Vec<Value> = rows.filter_map(|r| r.ok()).collect();
    json!(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_export_table_empty() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::schema::run_migrations(&conn).unwrap();
        let result = export_table(&conn, "profiles");
        assert_eq!(result, json!([]));
    }

    #[test]
    fn test_export_table_nonexistent() {
        let conn = Connection::open_in_memory().unwrap();
        let result = export_table(&conn, "nonexistent_table");
        assert_eq!(result, json!([]));
    }

    #[test]
    fn test_export_table_with_data() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::schema::run_migrations(&conn).unwrap();
        conn.execute(
            "INSERT INTO profiles (id, name, preferences) VALUES ('p1', 'Alice', '{}')",
            [],
        )
        .unwrap();
        let result = export_table(&conn, "profiles");
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["name"], "Alice");
    }
}
