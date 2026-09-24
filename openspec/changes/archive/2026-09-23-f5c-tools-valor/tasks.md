# F5c: Tools de Valor — Task Checklist

## TDD Tasks

- [x] **5c.1** Tool: Clima con coordenadas precisas (`src/tools/weather.rs`)
  - [x] RED: unit tests for WeatherTool (get_weather, error cases, permission)
  - [x] GREEN: implement WeatherTool with OpenWeather API calls
  - [x] REFACTOR: clippy, fmt

- [ ] **5c.2** Tool: Geocoding directo e inverso (`src/tools/geo.rs`)
  - [ ] RED: unit tests for GeoTool (geocode, reverse_geocode, error cases)
  - [ ] GREEN: implement GeoTool with Nominatim API calls
  - [ ] REFACTOR: clippy, fmt

- [ ] **5c.3** Tool: Búsqueda de lugares con radio (`src/tools/geo.rs`)
  - [ ] RED: unit tests for search_places
  - [ ] GREEN: implement search_places with Overpass OSM API
  - [ ] REFACTOR: clippy, fmt

- [ ] **5c.4** Tool: Menús semanales y lista de la compra (`src/tools/meals.rs`)
  - [ ] RED: unit tests for MealsTool (plan_week_meals, generate_shopping_list, add_to_shopping_list, list_shopping_list, check_off_item)
  - [ ] GREEN: implement MealsTool with repos
  - [ ] REFACTOR: clippy, fmt

- [ ] **5c.5** Tool: Seguimiento de hábitos y rachas (`src/tools/habits.rs`)
  - [ ] RED: unit tests for HabitsTool (create_habit, log_habit, habit_streaks, habit_stats)
  - [ ] GREEN: implement HabitsTool with repos
  - [ ] REFACTOR: clippy, fmt

- [x] **5c.6** Schema: nuevas tablas (meal_plans, shopping_list, habits, habit_logs)
  - [x] RED: migration tests for new tables
  - [x] GREEN: add CREATE TABLE statements to `src/db/schema.rs`
  - [x] REFACTOR: verify existing tests still pass

- [ ] **5c.7** Tests de integración
  - [ ] RED: integration tests for all 4 new tools
  - [ ] GREEN: ensure tools are registered in ToolRegistry
  - [ ] REFACTOR: clippy, fmt

- [ ] **5c.8** `cargo test`, `cargo clippy`, `cargo fmt`
  - [ ] Run full test suite
  - [ ] Run clippy with -D warnings
  - [ ] Run fmt --check