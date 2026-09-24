# Meals & Shopping Tool

## Contracts

```rust
pub struct MealsTool {
    db: Arc<Mutex<Connection>>,
}

pub struct MealPlan {
    pub id: String,
    pub profile_id: String,
    pub week_start: String,     // ISO date (Monday)
    pub meals: Value,           // JSON: { "monday": { "lunch": "...", "dinner": "..." }, ... }
    pub created_at: String,
}

pub struct ShoppingItem {
    pub id: String,
    pub profile_id: String,
    pub item: String,
    pub quantity: Option<String>,
    pub category: Option<String>,  // "verduras", "lácteos", "despensa", "carnes", "congelados"
    pub checked: bool,
    pub created_at: String,
}
```

## Scenarios

### Happy path: plan_week_meals
**Given** a profile with diet preferences exists  
**When** the user calls `plan_week_meals` with `{ "days": 5 }`  
**Then** the tool generates a meal plan for the specified days and stores it in `meal_plans`

### Happy path: plan_week_meals with preferences override
**Given** a profile exists  
**When** the user calls `plan_week_meals` with `{ "preferences": "sin gluten, vegetariana", "days": 3 }`  
**Then** the tool uses the override preferences instead of profile defaults

### Happy path: generate_shopping_list
**Given** a meal plan exists for the current week  
**When** the user calls `generate_shopping_list`  
**Then** the tool creates shopping items from meal ingredients, grouped by category

### Happy path: add_to_shopping_list
**Given** a valid item  
**When** the user calls `add_to_shopping_list` with `{ "item": "Leche", "quantity": "1L", "category": "lácteos" }`  
**Then** the item is added to the shopping list

### Happy path: list_shopping_list
**Given** items exist in the shopping list  
**When** the user calls `list_shopping_list`  
**Then** the tool returns all items, optionally filtered by category

### Happy path: check_off_item
**Given** an item exists in the shopping list  
**When** the user calls `check_off_item` with `{ "item": "Leche" }`  
**Then** the item is marked as checked

### Error: plan_week_meals without profile
**When** the user calls `plan_week_meals` without `profile_id`  
**Then** the tool returns `ToolError::InvalidArguments`

### Error: check_off_item not found
**When** the user calls `check_off_item` with a non-existent item  
**Then** the tool returns `ToolError::NotFound`

### Permission: plan_week_meals
**Given** the `plan_week_meals` operation  
**Then** its permission level is `Notify` (creates data)

### Permission: other operations
**Given** `add_to_shopping_list`, `list_shopping_list`, `check_off_item`, `generate_shopping_list`  
**Then** their permission level is `NoConfirm`