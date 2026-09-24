# Tasks — f1-scaffolding

## TDD Task Checklist

### Backend Rust

- [ ] **1.1** Crear Cargo.toml con todas las dependencias base
- [ ] **1.1** Crear src/main.rs con servidor Axum básico (escucha :3000)
- [ ] **1.1** Crear src/lib.rs (módulo raíz)
- [ ] **1.2** Configurar rutas: /api/health
- [ ] **1.2** Handler health que devuelve status + version + db connected
- [ ] **1.3** Conectar SQLite con rusqlite (bundled)
- [ ] **1.3** Registrar extensión sqlite-vec
- [ ] **1.4** Crear src/db/mod.rs con init_db()
- [ ] **1.4** Crear src/db/schema.rs con CREATE TABLE IF NOT EXISTS para todas las tablas
- [ ] **1.4** Migración auto al arrancar
- [ ] **1.7** Crear justfile con comandos básicos (dev, check, check-spec)

### Frontend React

- [ ] **1.5** Inicializar package.json con dependencias
- [ ] **1.5** Crear vite.config.ts con proxy
- [ ] **1.5** Crear tsconfig.json
- [ ] **1.5** Crear index.html
- [ ] **1.5** Crear src/main.tsx con React root
- [ ] **1.6** Crear src/theme.ts con tema Antd oscuro/claro
- [ ] **1.6** Crear src/App.tsx con ConfigProvider + AppLayout
- [ ] **1.6** Crear src/components/AppLayout.tsx con Antd Layout
- [ ] **1.7** Configurar ESLint

### Docker

- [x] **1.8** Crear Dockerfile.backend (multi-stage dev)
- [x] **1.8** Crear Dockerfile.frontend (dev)
- [x] **1.8** Crear docker-compose.yml (backend + frontend + volúmenes)

### Verification

- [ ] **1.9** `cargo build` pasa sin errores
- [ ] **1.9** `cargo test` pasa (tests vacíos OK)
- [ ] **1.9** `npm run build` pasa sin errores
- [ ] **1.9** `npx tsc --noEmit` pasa sin errores
- [ ] **1.9** GET /api/health responde 200

## RED Phase

1. Escribir tests que fallen:
   - `tests/api/health.rs`: test_integration_health_check
   - `tests/db/migrations.rs`: test_tables_exist, test_vector_version, test_migration_idempotent

2. Verificar que los tests fallan (no hay servidor ni DB)

## GREEN Phase

1. Implementar backend según spec
2. Implementar frontend según spec
3. Configurar Docker
4. Ejecutar `cargo build` + `npm run build`

## REFACTOR Phase

1. `cargo fmt --check`
2. `cargo clippy -- -D warnings`
3. `npx tsc --noEmit`
4. `npm run lint`
5. `just check-all` (cuando exista el justfile)