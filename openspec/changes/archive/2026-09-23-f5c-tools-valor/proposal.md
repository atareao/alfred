# F5c: Tools de Valor (Geo, Clima, Comidas, Hábitos)

## Intent

Añadir las 4 herramientas de valor que completan el ecosistema de Alfred:
- **Clima** (OpenWeather API) con coordenadas precisas
- **Geolocalización** (Nominatim OSM) — geocoding directo e inverso + búsqueda de lugares
- **Comidas y lista de la compra** — menús semanales según perfil dietético, generación automática de lista
- **Hábitos** — seguimiento de rachas y frecuencias

## Scope

| # | Tarea | Archivos |
|---|-------|----------|
| 5c.1 | Tool: Clima con coordenadas precisas | `src/tools/weather.rs` |
| 5c.2 | Tool: Geocoding directo e inverso (Nominatim) | `src/tools/geo.rs` |
| 5c.3 | Tool: Búsqueda de lugares con radio (OSM Overpass) | `src/tools/geo.rs` |
| 5c.4 | Tool: Menús semanales y lista de la compra | `src/tools/meals.rs`, `src/db/repos/meal_plans.rs`, `src/db/repos/shopping_list.rs` |
| 5c.5 | Tool: Seguimiento de hábitos y rachas | `src/tools/habits.rs`, `src/db/repos/habits.rs` |
| 5c.6 | Schema: nuevas tablas (meal_plans, shopping_list, habits, habit_logs) | `src/db/schema.rs` |
| 5c.7 | Tests de integración | `tests/api/tools.rs` |
| 5c.8 | `cargo test`, `cargo clippy`, `cargo fmt` | — |

## Impact

- **Nuevas dependencias externas**: reqwest ya está en Cargo.toml (usado por LLM providers). No se necesitan nuevas dependencias.
- **APIs externas**: OpenWeather (API key en env `OPENWEATHER_API_KEY`), Nominatim OSM (gratis, sin key), Overpass OSM (gratis, sin key).
- **Nuevas tablas SQLite**: `meal_plans`, `shopping_list`, `habits`, `habit_logs` — añadidas a `run_migrations` en `schema.rs`.
- **Seed de tools**: `weather`, `geo`, `meals`, `habits` ya están en `seed_defaults()`.
- **ToolRegistry**: las 4 nuevas tools se registran en `new_with_orchestrator()` en `lib.rs`.
- **Sin cambios en frontend**: las tools se exponen vía tool_calling del orquestador.