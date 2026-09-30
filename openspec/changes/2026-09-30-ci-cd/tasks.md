# Tasks — CI en GitHub Actions y deploy local verificable

### Fase 0 — Evidencia del fallo (rojo documentado)
`[LEGACY - INSPECT]`

- [ ] Confirmar que `Cargo.lock` no está trackeado: `git ls-files --error-unmatch Cargo.lock` → error
- [ ] Clonar en limpio (`git clone --depth 1 file:///data/rust/valet /tmp/opencode/valet-fresh`) y confirmar que `Cargo.lock` NO existe en el clon
- [ ] Confirmar que el `Dockerfile` lo requiere: `grep -n "COPY Cargo.toml Cargo.lock" Dockerfile`
- [ ] Confirmar la versión falsa: `curl -s localhost:3000/api/health` → `"version":"0.1.0"` frente a `grep -m1 '^version' Cargo.toml` → `0.9.0`
- [ ] Confirmar que el test congela el error: `grep -n 'json\["version"\]' tests/api/health.rs` → `"0.1.0"`

### Fase 1 — RED: el test de la versión
`[TDD - RED]`

- [ ] Cambiar `tests/api/health.rs` para afirmar `env!("CARGO_PKG_VERSION")` en vez del literal `"0.1.0"`
- [ ] `cargo test --test health` → SHALL fallar: el handler sigue devolviendo `0.1.0` y el paquete es `0.9.0`

### Fase 2 — GREEN: el handler deja de mentir
`[TDD - GREEN]`

- [ ] `src/lib.rs`: sustituir el literal `"0.1.0"` de `health_handler` por `env!("CARGO_PKG_VERSION")`
- [ ] `cargo test --test health` → en verde
- [ ] `cargo test` → suite completa en verde

### Fase 3 — Toolchain fijada
`[TDD - REFACTOR]`

- [ ] Crear `rust-toolchain.toml` con `channel = "1.98.1"` y `components = ["rustfmt", "clippy"]`
- [ ] `rustc --version` → confirmar que rustup instala 1.98.1 (cambio real en la máquina local, avisar al usuario)
- [ ] `Dockerfile`: `FROM docker.io/library/rust:alpine3.21` → `docker.io/library/rust:1.98.1-alpine3.21`
- [ ] `grep -n "^FROM" Dockerfile` → ningún tag móvil en el compilador

### Fase 4 — Workflows
`[TDD - GREEN]`

- [ ] Crear `.github/workflows/ci.yml` (jobs `backend` y `frontend`, disparadores `pull_request` y `push` a `development` y `main`, `concurrency` con `cancel-in-progress`)
- [ ] Crear `.github/workflows/image.yml` (`push` a `main` + `workflow_dispatch`, build con caché GHA y smoke test con `docker run` y sondeo de `/api/health`)
- [ ] Validar la sintaxis YAML de los dos ficheros en local
- [ ] Verificar que las versiones de las Actions usadas existen (no usar versiones de memoria)
- [ ] `image.yml`: construir SIN publicar, verificar por smoke test y publicar solo si pasa
- [ ] `image.yml`: declarar `permissions: packages: write` (el permiso por defecto del repo es `read`)
- [ ] `image.yml`: publicar con `docker/login-action@v4` contra `ghcr.io` usando `secrets.GITHUB_TOKEN`
- [ ] `image.yml`: tags `latest` y `sha-<corto>` en push a `main`; `vX.Y.Z`, `X.Y` y `latest` en tag `v*`
- [ ] `image.yml`: un disparo manual publica solo `sha-<corto>` y NO mueve `latest`
- [ ] `image.yml`: comprobar en el diseño que un push a `main` no puede publicar sin que haya pasado el smoke test

### Fase 5 — Cargo.lock e higiene de .gitignore
`[TDD - REFACTOR]`

- [ ] Quitar `Cargo.lock` de `.gitignore`
- [ ] `git add -f Cargo.lock` para que quede trackeado
- [ ] Separar la línea corrupta `*.db-shm*.tsbuildinfo` en dos reglas
- [ ] Añadir `frontend/.vitest/` al `.gitignore`
- [ ] `git rm --cached frontend/.vitest/json/output.json` y borrar el fichero del disco
- [ ] Confirmar con `git status` que no se cuela ningún `assets.svg` ni `temporal.svg`

### Fase 6 — Presupuesto de ESLint
`[TDD - GREEN]`

- [ ] Añadir `"lint:ci": "eslint . --max-warnings 11"` a `frontend/package.json`, dejando `lint` intacto
- [ ] `cd frontend && npm run lint:ci` → SHALL pasar con exactamente 11 warnings y 0 errores

### Fase 7 — Recetas just
`[TDD - GREEN]`

- [ ] Renombrar con `git mv justfile .justfile` para conservar el historial
- [ ] Confirmar que no queda ningún `justfile` ni `Justfile` en la raíz y que `git ls-files` muestra `.justfile`
- [ ] Confirmar con `just --list` que se resuelven las recetas del proyecto y NO las del `.justfile` del directorio padre
- [ ] NO reescribir las menciones a `justfile` de `openspec/changes/archive/`
- [ ] Añadir `build`, `deploy`, `health`, `frontend-lint` y `frontend-test` al `.justfile`
- [ ] `deploy` con bucle de healthcheck acotado y salida distinta de cero si no queda sano
- [ ] Ampliar `check-all` a `fmt clippy test frontend-lint frontend-test frontend-check`
- [ ] Actualizar `help`
- [ ] Confirmar que las seis recetas nuevas no existían antes y que no se modifica ninguna existente salvo `check-all`

### Fase 7b — Publicación y deploy desde GHCR

[OPENSPEC - ARCHIVE]
- [ ] `docker-compose.yml`: añadir `image: ghcr.io/atareao/valet-ai:latest` al servicio `valet`, conservando su `build:`
- [ ] `.justfile`: añadir `deploy` (pull + recrear + verificar) y `deploy-local` (build + recrear + verificar)
- [ ] Verificar empíricamente que `podman compose up -d` tras un `podman pull` NO reconstruye la imagen
- [ ] Confirmar que `just dev` sigue recompilando tras añadir `image:` al compose, porque pasa `--build` explícito (riesgo de regresión del ciclo de desarrollo)
- [ ] Comprobar con `podman inspect` que el contenedor corre la imagen publicada y no una local
- [ ] Confirmar tras el primer push a `main` que el paquete aparece en GHCR con los tags esperados
- [ ] Poner el paquete como público (por CLI si el token tiene scope; si no, hacerlo a mano en la interfaz de GitHub)

### Fase 8 — Rojo→verde del workflow de imagen (prueba real)
`[TDD - RED]`

- [ ] Push de la rama con `image.yml` pero SIN `Cargo.lock` trackeado todavía
- [ ] `gh workflow run image.yml --ref <rama>` → SHALL fallar con un error de `COPY` por `Cargo.lock`, dejando constancia del fallo que este cambio arregla
- [ ] Trackear `Cargo.lock` y volver a disparar → SHALL pasar
- [ ] Confirmar que el disparo manual desde la rama NO ha movido el tag `latest` en GHCR

### Fase 9 — Verificación local de todos los comandos del CI
`[TDD - REFACTOR]`

- [ ] `cargo fmt --all -- --check` → exit 0
- [ ] `cargo clippy --all-targets -- -D warnings` → exit 0
- [ ] `cargo test` → todo en verde
- [ ] `cd frontend && npm ci` → exit 0
- [ ] `cd frontend && npx tsc --noEmit` → exit 0
- [ ] `cd frontend && npm run lint:ci` → 11 warnings, 0 errores
- [ ] `cd frontend && npx vitest run` → 15 ficheros, 115 tests, 0 fallos
- [ ] `cd frontend && npm run build` → OK
- [ ] `just check-all` → verde y cubriendo frontend
- [ ] `just deploy` → servicio sano y versión reportada correcta
- [ ] `just deploy` → descarga la imagen publicada y NO compila nada en la máquina
- [ ] `just --list` desde la raíz muestra las recetas del proyecto tras el renombrado, incluidas `deploy` y `deploy-local`

### Fase 10 — PR y validación del CI en remoto
`[TDD - GREEN]`

- [ ] Branch `feature/ci-cd` desde `development`
- [ ] Commit (gitmoji + conventional commits) y push
- [ ] Abrir PR contra `development`
- [ ] Confirmar que los checks `backend` y `frontend` salen en verde en el PR: es la prueba de que el CI funciona
- [ ] Mergear a `development`
- [ ] Abrir PR de `development` a `main` y confirmar que el job de imagen corre y pasa

### Fase 11 — Cierre
`[OPENSPEC - ARCHIVE]`

- [ ] Marcar `tasks.md` y añadir `## Resultado` con los números reales
- [ ] Archivar el change: como en este repo no hay CLI de `openspec`, volcar los 9 requisitos del delta VERBATIM al final de `openspec/specs/infra/spec.md` (los `### Requirement:` deben coincidir carácter por carácter) y mover `openspec/changes/2026-09-30-ci-cd/` a `openspec/changes/archive/2026-09-30-ci-cd/`
- [ ] Corregir en `openspec/specs/infra/spec.md` el snippet del `Dockerfile` para que refleje la realidad: el `FROM` fijado y el `COPY` del binario `valet-reindex`, que hoy no aparece
- [ ] Confirmar que el delta lleva 10 requisitos (los 9 anteriores más el de publicación en GHCR)
- [ ] Confirmar que el delta cierra con 10 requisitos y 24 escenarios
- [ ] Confirmar que el delta cierra con 11 requisitos y 27 escenarios

### Pendiente de decisión del usuario (NO ejecutar sin OK)
`[OPENSPEC - WAITING]`

- [ ] Protección de rama en `development` y `main` exigiendo los checks del CI: es configuración del repo, cambia el flujo de trabajo, se decide después de ver el CI verde
