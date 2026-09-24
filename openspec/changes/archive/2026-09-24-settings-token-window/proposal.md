# Settings: ventana por tokens y system prompt configurables desde UI

## Why

Actualmente la ventana de contexto está hardcodeada a 12 mensajes, y el system prompt es un template fijo en código. El usuario quiere:
- Ventana configurable por **tokens** (no por mensajes)
- **System prompt** editable desde la UI
- Valores guardados en DB para persistencia entre reinicios

## What Changes

| # | Tarea | Archivos | Grupo |
|---|-------|----------|-------|
| 1 | Schema: tabla settings (key-value) + migración | `src/db/schema.rs` | A |
| 2 | Repo: SettingsRepo (get/set/delete) | `src/db/repos/settings.rs`, `src/db/repos/mod.rs` | A |
| 3 | API: GET/PUT /api/settings | `src/routes/settings.rs`, `src/lib.rs` (router) | B |
| 4 | SessionWindow: token-based con max_window_tokens | `src/orchestrator/session_window.rs` | C |
| 5 | Orchestrator: leer settings de DB al procesar mensaje | `src/orchestrator/agent.rs` | C |
| 6 | Frontend: página de Ajustes (tokens + prompt) | `frontend/src/...` | D |
| 7 | Tests de integración | Tests | E |

### 1. Schema (`src/db/schema.rs`)

```sql
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

Seed de valores por defecto:
```sql
INSERT OR IGNORE INTO settings (key, value) VALUES 
    ('max_window_tokens', '10000'),
    ('system_prompt', 'Eres Alfred...');
```

### 2. SettingsRepo (`src/db/repos/settings.rs`)

```rust
pub struct SettingsRepo;

impl SettingsRepo {
    pub fn get(conn: &Connection, key: &str) -> Result<Option<String>>;
    pub fn set(conn: &Connection, key: &str, value: &str) -> Result<()>;
    pub fn get_all(conn: &Connection) -> Result<HashMap<String, String>>;
    pub fn delete(conn: &Connection, key: &str) -> Result<()>;
}
```

### 3. API (`src/routes/settings.rs`)

- `GET /api/settings` → `{ "max_window_tokens": "10000", "system_prompt": "..." }`
- `PUT /api/settings` → recibe `{ "max_window_tokens": "10000", "system_prompt": "..." }`

### 4. SessionWindow (`src/orchestrator/session_window.rs`)

- Cambiar `max_window: usize` (mensajes) → `max_window_tokens: usize`
- Añadir `fn estimate_tokens(text: &str) -> usize` (aprox: chars / 2 para español)
- `compact()` elimina mensajes más antiguos hasta que el total estimado esté por debajo del threshold
- `rehydrate()` respeta el límite de tokens

### 5. Orchestrator (`src/orchestrator/agent.rs`)

Al procesar mensaje (ambos paths):
1. Leer settings de DB (`max_window_tokens`, `system_prompt`)
2. Sobrescribir `config.max_window_tokens` si viene de settings
3. Usar `system_prompt` de settings si existe, si no el del template
4. Pasar token limit al cargar historial del session window

### 6. Frontend

Nueva página/ruta `Settings` con:
- Input numérico para `max_window_tokens` (1000 - 100000)
- Textarea para `system_prompt`
- Botón "Guardar"
- Botón "Restaurar valores por defecto"

## Impact

- **DB**: nueva tabla `settings`, migración en `run_migrations()`
- **SessionWindow**: rompe compatibilidad (cambio de campo), pero solo se usa internamente
- **OrchestratorConfig**: se puede eliminar o ignorar `max_window` (ahora viene de DB)
- **Sin cambios en tools ni LLM providers**
- Frontend: nueva página, no rompe rutas existentes