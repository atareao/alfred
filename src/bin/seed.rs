use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = std::env::var("DATABASE_URL").unwrap_or_else(|_| "alfred.db".into());
    let pool = alfred::db::init_db(&db_path).await?;

    seed_profiles(&pool).await?;
    seed_events(&pool).await?;
    seed_tasks(&pool).await?;
    seed_notes(&pool).await?;
    seed_contacts(&pool).await?;
    seed_habits(&pool).await?;
    seed_meal_plans(&pool).await?;
    seed_shopping_list(&pool).await?;

    println!("✅ Seed data created in {}", db_path);
    Ok(())
}

async fn seed_profiles(db: &SqlitePool) -> Result<(), sqlx::Error> {
    let now = Utc::now().to_rfc3339();

    // Ana
    sqlx::query(
        "INSERT OR IGNORE INTO profiles (id, name, avatar_url, preferences, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .bind("profile-ana")
    .bind("Ana")
    .bind(Option::<String>::None)
    .bind("{}")
    .bind(&now)
    .bind(&now)
    .execute(db)
    .await?;

    // Luis
    sqlx::query(
        "INSERT OR IGNORE INTO profiles (id, name, avatar_url, preferences, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .bind("profile-luis")
    .bind("Luis")
    .bind(Option::<String>::None)
    .bind("{}")
    .bind(&now)
    .bind(&now)
    .execute(db)
    .await?;

    println!("  👤 Perfiles: Ana, Luis");
    Ok(())
}

async fn seed_events(db: &SqlitePool) -> Result<(), sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    let tomorrow = (Utc::now() + chrono::Duration::days(1)).to_rfc3339();
    let week_later = (Utc::now() + chrono::Duration::days(7)).to_rfc3339();

    sqlx::query(
        "INSERT OR IGNORE INTO events (id, profile_id, title, description, start_time, end_time, location, scope)
         VALUES (?1, 'profile-ana', 'Cena con amigos', 'Restaurante italiano', ?2, ?3, 'Trattoria Da Mario', 'shared')",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&now)
    .bind(&tomorrow)
    .execute(db)
    .await?;

    sqlx::query(
        "INSERT OR IGNORE INTO events (id, profile_id, title, description, start_time, end_time, location, scope)
         VALUES (?1, 'profile-ana', 'Revisión semanal', 'Revisar objetivos y tareas', ?2, ?3, 'Oficina', 'personal')",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&tomorrow)
    .bind(&week_later)
    .execute(db)
    .await?;

    println!("  📅 Eventos: Cena con amigos, Revisión semanal");
    Ok(())
}

async fn seed_tasks(db: &SqlitePool) -> Result<(), sqlx::Error> {
    for (content, priority, project, scope) in &[
        ("Comprar leche", "high", "Casa", "shared"),
        ("Llamar al seguro", "medium", "Administración", "personal"),
        ("Preparar presentación", "high", "Trabajo", "personal"),
        ("Leer artículo sobre IA", "low", "Formación", "shared"),
    ] {
        sqlx::query(
            "INSERT OR IGNORE INTO tasks (id, profile_id, content, status, priority, project, scope)
             VALUES (?1, 'profile-ana', ?2, 'pending', ?3, ?4, ?5)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(content)
        .bind(priority)
        .bind(project)
        .bind(scope)
        .execute(db)
        .await?;
    }

    // A completed task
    sqlx::query(
        "INSERT OR IGNORE INTO tasks (id, profile_id, content, status, priority, project, scope)
         VALUES (?1, 'profile-ana', 'Comprar pan', 'completed', 'low', 'Casa', 'shared')",
    )
    .bind(Uuid::new_v4().to_string())
    .execute(db)
    .await?;

    println!("  ✅ Tareas: 4 pendientes, 1 completada");
    Ok(())
}

async fn seed_notes(db: &SqlitePool) -> Result<(), sqlx::Error> {
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT OR IGNORE INTO notes (id, profile_id, content, category, tags, created_at)
         VALUES (?1, 'profile-ana', 'Idea para app: gestor de recetas con IA', 'idea', 'app,recetas', ?2)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&now)
    .execute(db)
    .await?;

    let yesterday = (Utc::now() - chrono::Duration::days(1)).to_rfc3339();
    sqlx::query(
        "INSERT OR IGNORE INTO notes (id, profile_id, content, category, tags, created_at)
         VALUES (?1, 'profile-ana', 'Hoy aprendí sobre sistemas de recomendación', 'journal', 'aprendizaje,IA', ?2)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&yesterday)
    .execute(db)
    .await?;

    println!("  📝 Notas: Idea app, Journal");
    Ok(())
}

async fn seed_contacts(db: &SqlitePool) -> Result<(), sqlx::Error> {
    for (name, phone, email) in &[
        ("María García", "+34 612 345 678", "maria@example.com"),
        ("Carlos López", "+34 698 765 432", "carlos@example.com"),
        ("Laura Martínez", "+34 655 123 456", "laura@example.com"),
    ] {
        sqlx::query(
            "INSERT OR IGNORE INTO contacts (id, profile_id, name, phone, email)
             VALUES (?1, 'profile-ana', ?2, ?3, ?4)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(name)
        .bind(phone)
        .bind(email)
        .execute(db)
        .await?;
    }
    println!("  👥 Contactos: María, Carlos, Laura");
    Ok(())
}

async fn seed_habits(db: &SqlitePool) -> Result<(), sqlx::Error> {
    let habit_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT OR IGNORE INTO habits (id, profile_id, name, frequency, target)
         VALUES (?1, 'profile-ana', 'Leer 20 minutos', 'daily', 20)",
    )
    .bind(&habit_id)
    .execute(db)
    .await?;

    // Log for today
    let today = Utc::now().format("%Y-%m-%d").to_string();
    sqlx::query(
        "INSERT OR IGNORE INTO habit_logs (habit_id, date, completed)
         VALUES (?1, ?2, 1)",
    )
    .bind(&habit_id)
    .bind(&today)
    .execute(db)
    .await?;

    // Weekly habit
    let weekly_habit_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT OR IGNORE INTO habits (id, profile_id, name, frequency, target)
         VALUES (?1, 'profile-ana', 'Hacer ejercicio', 'weekly', 3)",
    )
    .bind(&weekly_habit_id)
    .execute(db)
    .await?;

    println!("  🏃 Hábitos: Leer (daily), Ejercicio (weekly)");
    Ok(())
}

async fn seed_meal_plans(db: &SqlitePool) -> Result<(), sqlx::Error> {
    let week_start = Utc::now().format("%Y-%m-%d").to_string();
    let meals = serde_json::json!({
        "monday": { "lunch": "Lentejas con verduras", "dinner": "Crema de calabaza" },
        "tuesday": { "lunch": "Pollo al horno", "dinner": "Tortilla francesa" },
        "wednesday": { "lunch": "Ensalada de garbanzos", "dinner": "Pescado al vapor" },
        "thursday": { "lunch": "Arroz con verduras", "dinner": "Revuelto de setas" },
        "friday": { "lunch": "Pasta integral", "dinner": "Pizza casera" },
    });

    sqlx::query(
        "INSERT OR IGNORE INTO meal_plans (id, profile_id, week_start, meals)
         VALUES (?1, 'profile-ana', ?2, ?3)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&week_start)
    .bind(meals.to_string())
    .execute(db)
    .await?;
    println!("  🍽️  Menú semanal: 5 días planificados");
    Ok(())
}

async fn seed_shopping_list(db: &SqlitePool) -> Result<(), sqlx::Error> {
    for (item, quantity, category) in &[
        ("Leche", "1L", "lácteos"),
        ("Pan integral", "1 barra", "despensa"),
        ("Huevos", "12 uds", "huevos"),
        ("Espinacas", "200g", "verduras"),
        ("Pechuga de pollo", "500g", "carne"),
        ("Arroz integral", "1kg", "despensa"),
    ] {
        sqlx::query(
            "INSERT OR IGNORE INTO shopping_list (id, profile_id, item, quantity, category)
             VALUES (?1, 'profile-ana', ?2, ?3, ?4)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(item)
        .bind(quantity)
        .bind(category)
        .execute(db)
        .await?;
    }
    println!("  🛒 Lista de la compra: 6 artículos");
    Ok(())
}
