# Settings Token Window — Task Checklist

## TDD Tasks

### Grupo A: Backend — DB + Repo
- [ ] **1.1** RED: test que la migración crea tabla settings con valores por defecto
- [ ] **1.2** GREEN: schema migration + SettingsRepo (get/set/get_all/seed_defaults)
- [ ] **1.3** REFACTOR: `cargo clippy`, `cargo test`

### Grupo B: Backend — API
- [ ] **2.1** RED: test de integración GET /api/settings devuelve valores por defecto
- [ ] **2.2** RED: test PUT /api/settings actualiza valores
- [ ] **2.3** GREEN: endpoints GET/PUT /api/settings
- [ ] **2.4** REFACTOR: `cargo clippy`, `cargo test`

### Grupo C: Backend — SessionWindow token-based + Orchestrator
- [ ] **3.1** RED: test que SessionWindow respeta max_window_tokens
- [ ] **3.2** GREEN: migrar SessionWindow a token-based + estimate_tokens
- [ ] **3.3** RED: test que orchestrator lee settings de DB
- [ ] **3.4** GREEN: orchestrator usa system_prompt y max_window_tokens de settings
- [ ] **3.5** REFACTOR: `cargo clippy`, `cargo test`

### Grupo D: Frontend
- [ ] **4.1** Crear página Settings con formulario (tokens + prompt)
- [ ] **4.2** Conectar con API GET/PUT /api/settings
- [ ] **4.3** Añadir ruta /settings en el router del frontend
- [ ] **4.4** Verificar build sin errores

### Grupo E: Verificación final
- [ ] **5.1** `cargo test` — todos los tests pasan
- [ ] **5.2** `cargo clippy -- -D warnings` — cero warnings
- [ ] **5.3** `cargo fmt --check` — formato correcto
- [ ] **5.4** `npx tsc --noEmit && npm run build` — frontend compila