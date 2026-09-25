# TDD Task Checklist: geo-timeout-fix

## RED phase — Write failing tests first

- [x] **RED 1**: Test que `overpass_escape` solo escapa `\` y `"` (NO caracteres regex como `.` `(` `)` etc.)
  - `overpass_escape("cafe (bar)")` → `"cafe (bar)"` (sin cambios)
  - `overpass_escape("Mc.Donald's")` → `"Mc.Donald's"` (sin cambios)
  - `overpass_escape("cafe")` → `"cafe"`
  - `overpass_escape("\\")` → `"\\\\"`
  - `overpass_escape("\"")` → `"\\\""`
- [x] **RED 2**: Test que verifica que `GeoTool::new` configura timeout de 30s
  - Verificar que el cliente interno tiene `timeout` de 30s
- [x] **RED 3**: Test `search_places` con mock HTTP que devuelve 429 HTML → error incluye body raw `<html><body>Rate limited</body></html>`
- [x] **RED 4**: Test `search_places` con mock HTTP que devuelve 502 + JSON body → error incluye body raw, NO se parsea como resultado
- [x] **RED 5**: Test `search_places` con mock HTTP que devuelve 200 OK + JSON válido → `ToolResult.data` contiene el Value COMPLETO sin filtrar (incluye `elements`, `type`, `tags` anidados, etc.)

## GREEN phase — Implement

- [x] **GREEN 1**: `overpass_escape()` ya escapa solo `\` y `"` (no se necesita cambio)
- [x] **GREEN 2**: Timeout 30s configurado en `reqwest::Client::builder().timeout(...)`
- [x] **GREEN 3**: Body leído con `resp.text().await`, status verificado, parseo con `serde_json::from_str()`
- [x] **GREEN 4**: Status no-success → error con raw body
- [x] **GREEN 5**: Success → Value completo del body sin filtrar, eliminado bloque de extracción de places

## REFACTOR phase

- [x] **REFACTOR**: `cargo clippy -- -D warnings` — zero warnings
- [x] **REFACTOR**: `cargo test` — 414 tests green
- [x] **REFACTOR**: `cargo fmt --check` — sin diferencias