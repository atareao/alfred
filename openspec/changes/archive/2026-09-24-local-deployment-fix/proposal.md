# Local Deployment Fix: main.rs + docker-compose.yml

## Why

El `main.rs` actual hardcodea todos los valores: usa `"alfred.db"` como path de base de datos, no inicializa el orquestador ni los tools ni el LLM provider (todo es `None`), y bindea a `0.0.0.0:3000` fijo. Esto significa que el servidor arranca pero **no puede procesar mensajes de chat** porque el orquestador nunca se crea.

Además, `docker-compose.yml` (dev) no pasa las variables de entorno necesarias (`OPENROUTER_API_KEY`, `AUTH_ENABLED`) al backend.

## What Changes

| # | Tarea | Archivos | Grupo |
|---|-------|----------|-------|
| 1 | main.rs usa Config + new_with_orchestrator | `src/main.rs` | A |
| 2 | docker-compose.yml pasa env vars + volumen datos | `docker-compose.yml` | B |
| 3 | .env template para desarrollo local | `.env` | B |

### main.rs
- Importar `Config::from_env()` para leer configuración del entorno
- Usar `AppState::new_with_orchestrator()` en vez de montar AppState a mano con todo `None`
- Bindear a `config.host:config.port` en vez de `0.0.0.0:3000` hardcodeado
- Usar `config.log_level` para el filtro de tracing

### docker-compose.yml (dev)
- Añadir `OPENROUTER_API_KEY=${OPENROUTER_API_KEY}` y `OPENWEATHER_API_KEY=${OPENWEATHER_API_KEY}`
- Añadir `AUTH_ENABLED=false` (sin PocketID en dev)
- Añadir volumen `alfred_data` para persistencia de SQLite

### .env (nuevo)
- Template con variables esenciales para desarrollo local
- Auth deshabilitado por defecto
- OpenRouter como provider recomendado

## Impact

- **main.rs**: deja de ser código muerto — ahora inicializa todo correctamente
- **docker-compose.yml**: el dev compose funciona sin PocketID
- **Sin romper nada**: `app_with_state()` y `AppState::new_with_orchestrator()` ya existen y están probados