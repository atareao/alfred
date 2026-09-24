use async_trait::async_trait;
use chrono::Datelike;
use rusqlite::Connection;
use serde_json::Value;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::db::repos::meal_plans::MealPlansRepo;
use crate::db::repos::shopping_list::ShoppingListRepo;
use crate::tools::permission::Permission;
use crate::tools::r#trait::{Tool, ToolError, ToolResult};

pub struct MealsTool {
    db: Arc<Mutex<Connection>>,
}

const LUNCHES: &[&str] = &[
    "Lentejas con verduras",
    "Pollo al horno con patatas",
    "Pasta con pesto y tomates cherry",
    "Merluza a la plancha con ensalada",
    "Arroz con verduras y tofu",
    "Hamburguesas de lentejas",
    "Salteado de pollo con verduras",
];

const DINNERS: &[&str] = &[
    "Crema de calabaza",
    "Tortilla francesa con ensalada",
    "Bowl de quinoa y aguacate",
    "Sopa de verduras",
    "Hummus con crudités",
    "Pescado al papillote",
    "Revuelto de setas",
];

const DAY_NAMES: &[&str] = &[
    "lunes",
    "martes",
    "miércoles",
    "jueves",
    "viernes",
    "sábado",
    "domingo",
];

/// Maps a dish name to its ingredient list with categories.
fn dish_ingredients(dish: &str) -> Vec<(&str, &str)> {
    match dish {
        "Lentejas con verduras" => vec![
            ("Lentejas", "Legumbres"),
            ("Zanahoria", "Verduras"),
            ("Cebolla", "Verduras"),
            ("Pimiento", "Verduras"),
        ],
        "Pollo al horno con patatas" => vec![
            ("Pollo", "Carnes"),
            ("Patatas", "Verduras"),
            ("Aceite de oliva", "Aceites y especias"),
        ],
        "Pasta con pesto y tomates cherry" => vec![
            ("Pasta", "Pastas y arroces"),
            ("Albahaca", "Aceites y especias"),
            ("Tomates cherry", "Verduras"),
            ("Queso parmesano", "Lácteos"),
        ],
        "Merluza a la plancha con ensalada" => vec![
            ("Merluza", "Pescados"),
            ("Lechuga", "Verduras"),
            ("Tomate", "Verduras"),
            ("Limón", "Frutas"),
        ],
        "Arroz con verduras y tofu" => vec![
            ("Arroz", "Pastas y arroces"),
            ("Tofu", "Proteínas"),
            ("Calabacín", "Verduras"),
            ("Pimiento", "Verduras"),
        ],
        "Hamburguesas de lentejas" => vec![
            ("Lentejas", "Legumbres"),
            ("Pan de hamburguesa", "Panadería"),
            ("Lechuga", "Verduras"),
            ("Cebolla", "Verduras"),
        ],
        "Salteado de pollo con verduras" => vec![
            ("Pollo", "Carnes"),
            ("Brócoli", "Verduras"),
            ("Zanahoria", "Verduras"),
            ("Salsa de soja", "Aceites y especias"),
        ],
        "Crema de calabaza" => vec![
            ("Calabaza", "Verduras"),
            ("Cebolla", "Verduras"),
            ("Nata", "Lácteos"),
        ],
        "Tortilla francesa con ensalada" => vec![
            ("Huevos", "Proteínas"),
            ("Lechuga", "Verduras"),
            ("Tomate", "Verduras"),
        ],
        "Bowl de quinoa y aguacate" => vec![
            ("Quinoa", "Pastas y arroces"),
            ("Aguacate", "Frutas"),
            ("Tomate", "Verduras"),
            ("Maíz", "Verduras"),
        ],
        "Sopa de verduras" => vec![
            ("Zanahoria", "Verduras"),
            ("Apio", "Verduras"),
            ("Puerro", "Verduras"),
            ("Patatas", "Verduras"),
        ],
        "Hummus con crudités" => vec![
            ("Garbanzos", "Legumbres"),
            ("Tahini", "Aceites y especias"),
            ("Zanahoria", "Verduras"),
            ("Pepino", "Verduras"),
        ],
        "Pescado al papillote" => vec![
            ("Pescado blanco", "Pescados"),
            ("Limón", "Frutas"),
            ("Calabacín", "Verduras"),
        ],
        "Revuelto de setas" => vec![
            ("Huevos", "Proteínas"),
            ("Setas variadas", "Verduras"),
            ("Ajo", "Aceites y especias"),
        ],
        _ => vec![],
    }
}

impl MealsTool {
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self { db }
    }

    fn current_week_monday() -> String {
        let today = chrono::Utc::now().date_naive();
        let days_from_monday = today.weekday().num_days_from_monday();
        let monday = today - chrono::Duration::days(days_from_monday.into());
        monday.format("%Y-%m-%d").to_string()
    }

    async fn plan_week_meals(&self, args: Value) -> Result<ToolResult, ToolError> {
        let profile_id = args
            .get("profile_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing profile_id".into()))?;

        let preferences = args
            .get("preferences")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let days = args.get("days").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
        let days = days.clamp(1, 7);

        let start_index = chrono::Utc::now()
            .date_naive()
            .weekday()
            .num_days_from_monday() as usize;

        let mut meals_map = serde_json::Map::new();
        for i in 0..days {
            let day_name = DAY_NAMES.get(i).unwrap_or(&"desconocido");
            let lunch_idx = (start_index + i * 2) % LUNCHES.len();
            let dinner_idx = (start_index + i * 2 + 1) % DINNERS.len();
            let day_meals = serde_json::json!({
                "lunch": LUNCHES[lunch_idx],
                "dinner": DINNERS[dinner_idx],
            });
            meals_map.insert((*day_name).to_string(), day_meals);
        }

        let week_start = Self::current_week_monday();
        let meals_value = Value::Object(meals_map);

        let conn = self
            .db
            .lock()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        let plan = MealPlansRepo::upsert(&conn, profile_id, &week_start, &meals_value)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        let mut message = format!("Menú semanal creado para {}", week_start);
        if !preferences.is_empty() {
            message.push_str(&format!(" (preferencias: {})", preferences));
        }

        Ok(ToolResult {
            success: true,
            data: serde_json::to_value(&plan).unwrap_or_default(),
            message: Some(message),
        })
    }

    async fn generate_shopping_list(&self, args: Value) -> Result<ToolResult, ToolError> {
        let profile_id = args
            .get("profile_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing profile_id".into()))?;

        let conn = self
            .db
            .lock()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        let plan = MealPlansRepo::get_current(&conn, profile_id)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?
            .ok_or_else(|| ToolError::NotFound("No hay plan de comidas para esta semana".into()))?;

        let meals = &plan.meals;
        let mut all_ingredients: Vec<(&str, &str)> = Vec::new();

        if let Some(obj) = meals.as_object() {
            for day_meals in obj.values() {
                if let Some(lunch) = day_meals.get("lunch").and_then(|v| v.as_str()) {
                    all_ingredients.extend(dish_ingredients(lunch));
                }
                if let Some(dinner) = day_meals.get("dinner").and_then(|v| v.as_str()) {
                    all_ingredients.extend(dish_ingredients(dinner));
                }
            }
        }

        // Deduplicate by item name (case-insensitive)
        let mut seen = HashSet::new();
        let unique_ingredients: Vec<_> = all_ingredients
            .into_iter()
            .filter(|(item, _)| seen.insert(item.to_lowercase()))
            .collect();

        // Get existing items to avoid duplicates in the DB
        let existing_items = ShoppingListRepo::list(&conn, profile_id, None)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;
        let existing_names: HashSet<String> = existing_items
            .iter()
            .map(|i| i.item.to_lowercase())
            .collect();

        let mut added_items = Vec::new();
        for (item, category) in &unique_ingredients {
            if !existing_names.contains(&item.to_lowercase()) {
                let added = ShoppingListRepo::add(&conn, profile_id, item, None, Some(category))
                    .map_err(|e| ToolError::ExecutionError(e.to_string()))?;
                added_items.push(added);
            }
        }

        Ok(ToolResult {
            success: true,
            data: serde_json::to_value(&added_items).unwrap_or_default(),
            message: Some(format!(
                "Añadidos {} ingredientes a la lista de la compra",
                added_items.len()
            )),
        })
    }

    async fn add_to_shopping_list(&self, args: Value) -> Result<ToolResult, ToolError> {
        let profile_id = args
            .get("profile_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing profile_id".into()))?;
        let item = args
            .get("item")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing item".into()))?;
        let quantity = args.get("quantity").and_then(|v| v.as_str());
        let category = args.get("category").and_then(|v| v.as_str());

        let conn = self
            .db
            .lock()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        let shopping_item = ShoppingListRepo::add(&conn, profile_id, item, quantity, category)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        Ok(ToolResult {
            success: true,
            data: serde_json::to_value(&shopping_item).unwrap_or_default(),
            message: Some(format!("Añadido '{}' a la lista de la compra", item)),
        })
    }

    async fn list_shopping_list(&self, args: Value) -> Result<ToolResult, ToolError> {
        let profile_id = args
            .get("profile_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing profile_id".into()))?;
        let category = args.get("category").and_then(|v| v.as_str());

        let conn = self
            .db
            .lock()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        let items = ShoppingListRepo::list(&conn, profile_id, category)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        Ok(ToolResult {
            success: true,
            data: serde_json::to_value(&items).unwrap_or_default(),
            message: Some(format!("{} artículos en la lista", items.len())),
        })
    }

    async fn check_off_item(&self, args: Value) -> Result<ToolResult, ToolError> {
        let profile_id = args
            .get("profile_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing profile_id".into()))?;
        let item = args
            .get("item")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing item".into()))?;

        let conn = self
            .db
            .lock()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        let updated = ShoppingListRepo::check_off(&conn, profile_id, item)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        if !updated {
            return Err(ToolError::NotFound(format!(
                "Item '{}' no encontrado en la lista de la compra",
                item
            )));
        }

        Ok(ToolResult {
            success: true,
            data: serde_json::json!({"item": item, "checked": true}),
            message: Some(format!("'{}' marcado como comprado", item)),
        })
    }
}

#[async_trait]
impl Tool for MealsTool {
    fn name(&self) -> &'static str {
        "meals"
    }

    fn description(&self) -> &'static str {
        "Planificación de comidas semanales y gestión de la lista de la compra"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": [
                        "plan_week_meals",
                        "generate_shopping_list",
                        "add_to_shopping_list",
                        "list_shopping_list",
                        "check_off_item"
                    ]
                },
                "profile_id": { "type": "string", "description": "Profile ID" },
                "preferences": { "type": "string", "description": "Preferencias dietéticas (opcional)" },
                "days": { "type": "integer", "description": "Número de días a planificar (1-7, por defecto 5)", "minimum": 1, "maximum": 7 },
                "item": { "type": "string", "description": "Nombre del artículo" },
                "quantity": { "type": "string", "description": "Cantidad (opcional)" },
                "category": { "type": "string", "description": "Categoría del artículo (opcional)" }
            },
            "required": ["operation"]
        })
    }

    fn permission(&self) -> Permission {
        Permission::Notify
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, ToolError> {
        let operation = args.get("operation").and_then(|v| v.as_str()).unwrap_or("");

        match operation {
            "plan_week_meals" => self.plan_week_meals(args).await,
            "generate_shopping_list" => self.generate_shopping_list(args).await,
            "add_to_shopping_list" => self.add_to_shopping_list(args).await,
            "list_shopping_list" => self.list_shopping_list(args).await,
            "check_off_item" => self.check_off_item(args).await,
            _ => Err(ToolError::InvalidArguments(format!(
                "Unknown operation: {}",
                operation
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::run_migrations;

    fn setup_db() -> Arc<Mutex<Connection>> {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn.execute(
            "INSERT INTO profiles (id, name, preferences) VALUES ('profile-1', 'Test', '{}')",
            [],
        )
        .unwrap();
        Arc::new(Mutex::new(conn))
    }

    #[tokio::test]
    async fn test_meals_name_and_description() {
        let db = setup_db();
        let tool = MealsTool::new(db);
        assert_eq!(tool.name(), "meals");
        assert_eq!(
            tool.description(),
            "Planificación de comidas semanales y gestión de la lista de la compra"
        );
    }

    #[tokio::test]
    async fn test_meals_permission() {
        let db = setup_db();
        let tool = MealsTool::new(db);
        assert_eq!(tool.permission(), Permission::Notify);
    }

    #[tokio::test]
    async fn test_meals_parameters_has_operations() {
        let db = setup_db();
        let tool = MealsTool::new(db);
        let params = tool.parameters();
        assert_eq!(params["type"], "object");
        assert!(params.get("properties").is_some());
        assert!(params.get("required").is_some());
        assert_eq!(params["required"][0], "operation");
        let ops = params["properties"]["operation"]["enum"]
            .as_array()
            .unwrap();
        let op_names: Vec<&str> = ops.iter().map(|v| v.as_str().unwrap()).collect();
        assert!(op_names.contains(&"plan_week_meals"));
        assert!(op_names.contains(&"generate_shopping_list"));
        assert!(op_names.contains(&"add_to_shopping_list"));
        assert!(op_names.contains(&"list_shopping_list"));
        assert!(op_names.contains(&"check_off_item"));
    }

    #[tokio::test]
    async fn test_plan_week_meals_missing_profile() {
        let db = setup_db();
        let tool = MealsTool::new(db);
        let err = tool
            .execute(serde_json::json!({
                "operation": "plan_week_meals"
            }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidArguments(_)));
    }

    #[tokio::test]
    async fn test_add_to_shopping_list_missing_item() {
        let db = setup_db();
        let tool = MealsTool::new(db);
        let err = tool
            .execute(serde_json::json!({
                "operation": "add_to_shopping_list",
                "profile_id": "profile-1"
            }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidArguments(_)));
    }

    #[tokio::test]
    async fn test_check_off_item_not_found() {
        let db = setup_db();
        let tool = MealsTool::new(db);
        let err = tool
            .execute(serde_json::json!({
                "operation": "check_off_item",
                "profile_id": "profile-1",
                "item": "Inexistente"
            }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_invalid_operation() {
        let db = setup_db();
        let tool = MealsTool::new(db);
        let err = tool
            .execute(serde_json::json!({
                "operation": "nonexistent"
            }))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidArguments(_)));
    }

    #[tokio::test]
    async fn test_plan_week_meals_creates_plan() {
        let db = setup_db();
        let tool = MealsTool::new(db);

        let result = tool
            .execute(serde_json::json!({
                "operation": "plan_week_meals",
                "profile_id": "profile-1"
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.data["profile_id"], "profile-1");
        // The meals field should contain days with lunch and dinner
        let meals = result.data["meals"].as_object().unwrap();
        assert!(meals.contains_key("lunes"));
        assert!(meals.contains_key("martes"));
        assert!(meals.contains_key("miércoles"));
        assert!(meals.contains_key("jueves"));
        assert!(meals.contains_key("viernes"));
        // Verify structure of a day
        let monday = &meals["lunes"];
        assert!(monday.get("lunch").and_then(|v| v.as_str()).is_some());
        assert!(monday.get("dinner").and_then(|v| v.as_str()).is_some());
    }

    #[tokio::test]
    async fn test_plan_week_meals_with_custom_days() {
        let db = setup_db();
        let tool = MealsTool::new(db);

        let result = tool
            .execute(serde_json::json!({
                "operation": "plan_week_meals",
                "profile_id": "profile-1",
                "days": 3
            }))
            .await
            .unwrap();

        assert!(result.success);
        let meals = result.data["meals"].as_object().unwrap();
        assert_eq!(meals.len(), 3);
        assert!(meals.contains_key("lunes"));
        assert!(meals.contains_key("martes"));
        assert!(meals.contains_key("miércoles"));
        assert!(!meals.contains_key("jueves"));
    }

    #[tokio::test]
    async fn test_add_and_list_shopping() {
        let db = setup_db();
        let tool = MealsTool::new(db);

        // Add an item
        let add_result = tool
            .execute(serde_json::json!({
                "operation": "add_to_shopping_list",
                "profile_id": "profile-1",
                "item": "Leche",
                "quantity": "1 litro",
                "category": "Lácteos"
            }))
            .await
            .unwrap();

        assert!(add_result.success);
        assert_eq!(add_result.data["item"], "Leche");
        assert_eq!(add_result.data["quantity"], "1 litro");
        assert_eq!(add_result.data["category"], "Lácteos");

        // List shopping list
        let list_result = tool
            .execute(serde_json::json!({
                "operation": "list_shopping_list",
                "profile_id": "profile-1"
            }))
            .await
            .unwrap();

        assert!(list_result.success);
        let items = list_result.data.as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["item"], "Leche");
    }

    #[tokio::test]
    async fn test_generate_shopping_list_no_plan() {
        let db = setup_db();
        let tool = MealsTool::new(db);

        let err = tool
            .execute(serde_json::json!({
                "operation": "generate_shopping_list",
                "profile_id": "profile-1"
            }))
            .await
            .unwrap_err();

        assert!(matches!(err, ToolError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_generate_shopping_list_with_plan() {
        let db = setup_db();
        let tool = MealsTool::new(db);

        // First create a meal plan
        tool.execute(serde_json::json!({
            "operation": "plan_week_meals",
            "profile_id": "profile-1",
            "days": 2
        }))
        .await
        .unwrap();

        // Generate shopping list from the plan
        let result = tool
            .execute(serde_json::json!({
                "operation": "generate_shopping_list",
                "profile_id": "profile-1"
            }))
            .await
            .unwrap();

        assert!(result.success);
        let items = result.data.as_array().unwrap();
        assert!(!items.is_empty(), "Should have generated ingredients");
        // Each item should have a category
        for item in items {
            assert!(item.get("category").and_then(|v| v.as_str()).is_some());
        }
    }

    #[tokio::test]
    async fn test_check_off_item_success() {
        let db = setup_db();
        let tool = MealsTool::new(db);

        // Add an item first
        tool.execute(serde_json::json!({
            "operation": "add_to_shopping_list",
            "profile_id": "profile-1",
            "item": "Pan",
            "category": "Panadería"
        }))
        .await
        .unwrap();

        // Check it off
        let result = tool
            .execute(serde_json::json!({
                "operation": "check_off_item",
                "profile_id": "profile-1",
                "item": "Pan"
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.data["item"], "Pan");
        assert_eq!(result.data["checked"], true);
    }
}
