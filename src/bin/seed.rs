use chrono::Utc;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = std::env::var("DATABASE_URL").unwrap_or_else(|_| "alfred.db".into());
    let conn = alfred::db::init_db(&db_path)?;
    let db = Arc::new(Mutex::new(conn));

    seed_profiles(&db)?;
    seed_conversations(&db)?;
    seed_events(&db)?;
    seed_tasks(&db)?;
    seed_notes(&db)?;
    seed_contacts(&db)?;
    seed_habits(&db)?;
    seed_meal_plans(&db)?;
    seed_shopping_list(&db)?;

    println!("✅ Seed data created in {}", db_path);
    Ok(())
}

fn seed_profiles(db: &Arc<Mutex<Connection>>) -> Result<(), rusqlite::Error> {
    let conn = db.lock().unwrap();
    let now = Utc::now().to_rfc3339();

    // Ana
    conn.execute(
        "INSERT OR IGNORE INTO profiles (id, name, avatar_url, preferences, created_at, updated_at)
         VALUES ('profile-ana', 'Ana', NULL, '{}', ?1, ?1)",
        rusqlite::params![now],
    )?;

    // Luis
    conn.execute(
        "INSERT OR IGNORE INTO profiles (id, name, avatar_url, preferences, created_at, updated_at)
         VALUES ('profile-luis', 'Luis', NULL, '{}', ?1, ?1)",
        rusqlite::params![now],
    )?;

    println!("  👤 Perfiles: Ana, Luis");
    Ok(())
}

fn seed_conversations(db: &Arc<Mutex<Connection>>) -> Result<(), rusqlite::Error> {
    let conn = db.lock().unwrap();
    let now = Utc::now().to_rfc3339();

    conn.execute(
        "INSERT OR IGNORE INTO conversations (id, title, created_at, updated_at)
         VALUES ('conv-main', '💬 Conversación principal', ?1, ?1)",
        rusqlite::params![now],
    )?;

    println!("  💬 Conversación principal creada");
    Ok(())
}

fn seed_events(db: &Arc<Mutex<Connection>>) -> Result<(), rusqlite::Error> {
    let conn = db.lock().unwrap();
    let now = Utc::now().to_rfc3339();
    let tomorrow = (Utc::now() + chrono::Duration::days(1)).to_rfc3339();
    let week_later = (Utc::now() + chrono::Duration::days(7)).to_rfc3339();

    conn.execute(
        "INSERT OR IGNORE INTO events (id, profile_id, title, description, start_time, end_time, location, scope)
         VALUES (?1, 'profile-ana', 'Cena con amigos', 'Restaurante italiano', ?2, ?3, 'Trattoria Da Mario', 'shared')",
        rusqlite::params![Uuid::new_v4().to_string(), now, tomorrow],
    )?;

    conn.execute(
        "INSERT OR IGNORE INTO events (id, profile_id, title, description, start_time, end_time, location, scope)
         VALUES (?1, 'profile-ana', 'Revisión semanal', 'Revisar objetivos y tareas', ?2, ?3, 'Oficina', 'personal')",
        rusqlite::params![Uuid::new_v4().to_string(), tomorrow, week_later],
    )?;

    println!("  📅 Eventos: Cena con amigos, Revisión semanal");
    Ok(())
}

fn seed_tasks(db: &Arc<Mutex<Connection>>) -> Result<(), rusqlite::Error> {
    let conn = db.lock().unwrap();

    for (content, priority, project, scope) in &[
        ("Comprar leche", "high", "Casa", "shared"),
        ("Llamar al seguro", "medium", "Administración", "personal"),
        ("Preparar presentación", "high", "Trabajo", "personal"),
        ("Leer artículo sobre IA", "low", "Formación", "shared"),
    ] {
        conn.execute(
            "INSERT OR IGNORE INTO tasks (id, profile_id, content, status, priority, project, scope)
             VALUES (?1, 'profile-ana', ?2, 'pending', ?3, ?4, ?5)",
            rusqlite::params![Uuid::new_v4().to_string(), content, priority, project, scope],
        )?;
    }

    // A completed task
    conn.execute(
        "INSERT OR IGNORE INTO tasks (id, profile_id, content, status, priority, project, scope)
         VALUES (?1, 'profile-ana', 'Comprar pan', 'completed', 'low', 'Casa', 'shared')",
        rusqlite::params![Uuid::new_v4().to_string()],
    )?;

    println!("  ✅ Tareas: 4 pendientes, 1 completada");
    Ok(())
}

fn seed_notes(db: &Arc<Mutex<Connection>>) -> Result<(), rusqlite::Error> {
    let conn = db.lock().unwrap();
    let now = Utc::now().to_rfc3339();

    conn.execute(
        "INSERT OR IGNORE INTO notes (id, profile_id, content, category, tags, created_at)
         VALUES (?1, 'profile-ana', 'Idea para app: gestor de recetas con IA', 'idea', 'app,recetas', ?2)",
        rusqlite::params![Uuid::new_v4().to_string(), now],
    )?;

    let yesterday = (Utc::now() - chrono::Duration::days(1)).to_rfc3339();
    conn.execute(
        "INSERT OR IGNORE INTO notes (id, profile_id, content, category, tags, created_at)
         VALUES (?1, 'profile-ana', 'Hoy aprendí sobre sistemas de recomendación', 'journal', 'aprendizaje,IA', ?2)",
        rusqlite::params![Uuid::new_v4().to_string(), yesterday],
    )?;

    println!("  📝 Notas: Idea app, Journal");
    Ok(())
}

fn seed_contacts(db: &Arc<Mutex<Connection>>) -> Result<(), rusqlite::Error> {
    let conn = db.lock().unwrap();

    for (name, phone, email) in &[
        ("María García", "+34 612 345 678", "maria@example.com"),
        ("Carlos López", "+34 698 765 432", "carlos@example.com"),
        ("Laura Martínez", "+34 655 123 456", "laura@example.com"),
    ] {
        conn.execute(
            "INSERT OR IGNORE INTO contacts (id, profile_id, name, phone, email)
             VALUES (?1, 'profile-ana', ?2, ?3, ?4)",
            rusqlite::params![Uuid::new_v4().to_string(), name, phone, email],
        )?;
    }
    println!("  👥 Contactos: María, Carlos, Laura");
    Ok(())
}

fn seed_habits(db: &Arc<Mutex<Connection>>) -> Result<(), rusqlite::Error> {
    let conn = db.lock().unwrap();

    let habit_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT OR IGNORE INTO habits (id, profile_id, name, frequency, target)
         VALUES (?1, 'profile-ana', 'Leer 20 minutos', 'daily', 20)",
        rusqlite::params![habit_id],
    )?;

    // Log for today
    let today = Utc::now().format("%Y-%m-%d").to_string();
    conn.execute(
        "INSERT OR IGNORE INTO habit_logs (habit_id, date, completed)
         VALUES (?1, ?2, 1)",
        rusqlite::params![habit_id, today],
    )?;

    // Weekly habit
    let weekly_habit_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT OR IGNORE INTO habits (id, profile_id, name, frequency, target)
         VALUES (?1, 'profile-ana', 'Hacer ejercicio', 'weekly', 3)",
        rusqlite::params![weekly_habit_id],
    )?;

    println!("  🏃 Hábitos: Leer (daily), Ejercicio (weekly)");
    Ok(())
}

fn seed_meal_plans(db: &Arc<Mutex<Connection>>) -> Result<(), rusqlite::Error> {
    let conn = db.lock().unwrap();
    let week_start = Utc::now().format("%Y-%m-%d").to_string();
    let meals = serde_json::json!({
        "monday": { "lunch": "Lentejas con verduras", "dinner": "Crema de calabaza" },
        "tuesday": { "lunch": "Pollo al horno", "dinner": "Tortilla francesa" },
        "wednesday": { "lunch": "Ensalada de garbanzos", "dinner": "Pescado al vapor" },
        "thursday": { "lunch": "Arroz con verduras", "dinner": "Revuelto de setas" },
        "friday": { "lunch": "Pasta integral", "dinner": "Pizza casera" },
    });

    conn.execute(
        "INSERT OR IGNORE INTO meal_plans (id, profile_id, week_start, meals)
         VALUES (?1, 'profile-ana', ?2, ?3)",
        rusqlite::params![Uuid::new_v4().to_string(), week_start, meals.to_string()],
    )?;
    println!("  🍽️  Menú semanal: 5 días planificados");
    Ok(())
}

fn seed_shopping_list(db: &Arc<Mutex<Connection>>) -> Result<(), rusqlite::Error> {
    let conn = db.lock().unwrap();

    for (item, quantity, category) in &[
        ("Leche", "1L", "lácteos"),
        ("Pan integral", "1 barra", "despensa"),
        ("Huevos", "12 uds", "huevos"),
        ("Espinacas", "200g", "verduras"),
        ("Pechuga de pollo", "500g", "carne"),
        ("Arroz integral", "1kg", "despensa"),
    ] {
        conn.execute(
            "INSERT OR IGNORE INTO shopping_list (id, profile_id, item, quantity, category)
             VALUES (?1, 'profile-ana', ?2, ?3, ?4)",
            rusqlite::params![Uuid::new_v4().to_string(), item, quantity, category],
        )?;
    }
    println!("  🛒 Lista de la compra: 6 artículos");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};

    fn setup_db() -> Arc<Mutex<Connection>> {
        let conn = Connection::open_in_memory().unwrap();
        alfred::db::schema::run_migrations(&conn).unwrap();
        let _ = alfred::db::vector::register_vector_ext(&conn);
        let _ = alfred::db::fts::create_fts_triggers(&conn);
        let _ = alfred::db::repos::tools::ToolsRepo::seed_defaults(&conn);
        Arc::new(Mutex::new(conn))
    }

    #[test]
    fn test_seed_profiles_creates_two() {
        let db = setup_db();
        seed_profiles(&db).unwrap();
        let conn = db.lock().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM profiles", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_seed_profiles_is_idempotent() {
        let db = setup_db();
        seed_profiles(&db).unwrap();
        seed_profiles(&db).unwrap();
        let conn = db.lock().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM profiles", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 2, "Profiles should not be duplicated");
    }

    #[test]
    fn test_seed_tasks_creates_multiple() {
        let db = setup_db();
        seed_profiles(&db).unwrap();
        seed_tasks(&db).unwrap();
        let conn = db.lock().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))
            .unwrap();
        assert!(count > 0, "Expected at least 1 task");
        // Should have both pending and completed
        let pending: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tasks WHERE status = 'pending'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let completed: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tasks WHERE status = 'completed'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(pending > 0, "Expected pending tasks");
        assert!(completed > 0, "Expected a completed task");
    }

    #[test]
    fn test_seed_contacts_creates_multiple() {
        let db = setup_db();
        seed_profiles(&db).unwrap();
        seed_contacts(&db).unwrap();
        let conn = db.lock().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM contacts", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_seed_habits_creates_with_logs() {
        let db = setup_db();
        seed_profiles(&db).unwrap();
        seed_habits(&db).unwrap();
        let conn = db.lock().unwrap();
        let habits: i64 = conn
            .query_row("SELECT COUNT(*) FROM habits", [], |row| row.get(0))
            .unwrap();
        assert_eq!(habits, 2, "Expected 2 habits (daily + weekly)");
        let logs: i64 = conn
            .query_row("SELECT COUNT(*) FROM habit_logs", [], |row| row.get(0))
            .unwrap();
        assert!(logs > 0, "Expected at least 1 habit log");
    }

    #[test]
    fn test_seed_shopping_list_creates_items() {
        let db = setup_db();
        seed_profiles(&db).unwrap();
        seed_shopping_list(&db).unwrap();
        let conn = db.lock().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM shopping_list", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 6);
    }

    #[test]
    fn test_seed_events_creates_events() {
        let db = setup_db();
        seed_profiles(&db).unwrap();
        seed_events(&db).unwrap();
        let conn = db.lock().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 2, "Expected 2 events");
    }

    #[test]
    fn test_seed_notes_creates_notes() {
        let db = setup_db();
        seed_profiles(&db).unwrap();
        seed_notes(&db).unwrap();
        let conn = db.lock().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM notes", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 2, "Expected 2 notes");
    }

    #[test]
    fn test_seed_all_together() {
        let db = setup_db();
        let conn = db.lock().unwrap();
        drop(conn);

        seed_profiles(&db).unwrap();
        seed_conversations(&db).unwrap();
        seed_events(&db).unwrap();
        seed_tasks(&db).unwrap();
        seed_notes(&db).unwrap();
        seed_contacts(&db).unwrap();
        seed_habits(&db).unwrap();
        seed_meal_plans(&db).unwrap();
        seed_shopping_list(&db).unwrap();

        let conn = db.lock().unwrap();
        for (table, expected_min) in &[
            ("profiles", 2i64),
            ("conversations", 1),
            ("events", 2),
            ("tasks", 5),
            ("notes", 2),
            ("contacts", 3),
            ("habits", 2),
            ("habit_logs", 1),
            ("shopping_list", 6),
            ("meal_plans", 1),
        ] {
            let count: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {}", table), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert!(
                count >= *expected_min,
                "Expected at least {} rows in {}, got {}",
                expected_min,
                table,
                count
            );
        }
    }
}
