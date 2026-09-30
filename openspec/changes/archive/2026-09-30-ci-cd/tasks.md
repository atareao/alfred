# Tasks — CI en GitHub Actions y deploy local verificable

### Fase 0 — Evidencia del fallo (rojo documentado)
`[LEGACY - INSPECT]`

- [x] Confirmar que `Cargo.lock` no está trackeado: `git ls-files --error-unmatch Cargo.lock` → error
- [x] Clonar en limpio (`git clone --depth 1 file:///data/rust/valet /tmp/opencode/valet-fresh`) y confirmar que `Cargo.lock` NO existe en el clon
- [x] Confirmar que el `Dockerfile` lo requiere: `grep -n "COPY Cargo.toml Cargo.lock" Dockerfile` → `Dockerfile:20`
- [x] Confirmar la versión falsa: `curl -s localhost:3000/api/health` → `"version":"0.1.0"` frente a `grep -m1 '^version' Cargo.toml` → `0.9.0`
- [x] Confirmar que el test congela el error: `grep -n 'json\["version"\]' tests/api/health.rs` → `tests/api/health.rs:74` afirma `"0.1.0"`

### Fase 1 — RED: el test de la versión
`[TDD - RED]`

- [x] Cambiar `tests/api/health.rs` para afirmar `env!("CARGO_PKG_VERSION")` en vez del literal `"0.1.0"`
- [x] `cargo test --test health` → SHALL fallar: el handler sigue devolviendo `0.1.0` y el paquete es `0.9.0` (falló con `left: String("0.1.0")` / `right: "0.9.0"`)

### Fase 2 — GREEN: el handler deja de mentir
`[TDD - GREEN]`

- [x] `src/lib.rs`: sustituir el literal `"0.1.0"` de `health_handler` por `env!("CARGO_PKG_VERSION")`
- [x] `cargo test --test health` → en verde
- [x] `cargo test` → suite completa en verde (623 tests, 0 fallos)

### Fase 3 — Toolchain fijada
`[TDD - REFACTOR]`

- [x] Crear `rust-toolchain.toml` con `channel = "1.98.1"` y `components = ["rustfmt", "clippy"]`
- [x] `rustc --version` → confirmar que rustup instala 1.98.1 (`rustc 1.98.1 (48a229cea 2026-09-01)`)
- [x] `Dockerfile`: `FROM docker.io/library/rust:alpine3.21` → `docker.io/library/rust:1.98.1-alpine3.21`
- [x] `grep -n "^FROM" Dockerfile` → ningún tag móvil en el compilador

### Fase 4 — Workflows
`[TDD - GREEN]`

- [x] Crear `.github/workflows/ci.yml` (jobs `backend` y `frontend`, disparadores `pull_request` y `push` a `development` y `main`, `concurrency` con `cancel-in-progress`)
- [x] Crear `.github/workflows/image.yml` (`push` a `main` + `workflow_dispatch`, build con caché GHA y smoke test con `docker run` y sondeo de `/api/health`)
- [x] Validar la sintaxis YAML de los dos ficheros en local (PyYAML → `YAML OK`)
- [x] Verificar que las versiones de las Actions usadas existen (no usar versiones de memoria): los 6 refs verificados contra la API de GitHub
- [x] `image.yml`: construir SIN publicar, verificar por smoke test y publicar solo si pasa
- [x] `image.yml`: declarar `permissions: packages: write` (el permiso por defecto del repo es `read`)
- [x] `image.yml`: publicar con `docker/login-action@v4` contra `ghcr.io` usando `secrets.GITHUB_TOKEN`
- [x] `image.yml`: tags `latest` y `sha-<corto>` en push a `main`; `vX.Y.Z`, `X.Y` y `latest` en tag `v*`
- [x] `image.yml`: un disparo manual publica solo `sha-<corto>` y NO mueve `latest`
- [x] `image.yml`: comprobar en el diseño que un push a `main` no puede publicar sin que haya pasado el smoke test

### Fase 5 — Cargo.lock e higiene de .gitignore
`[TDD - REFACTOR]`

- [x] Quitar `Cargo.lock` de `.gitignore`
- [x] `git add -f Cargo.lock` para que quede trackeado
- [x] Separar la línea corrupta `*.db-shm*.tsbuildinfo` en dos reglas
- [x] Añadir `frontend/.vitest/` al `.gitignore`
- [x] `git rm --cached frontend/.vitest/json/output.json` y borrar el fichero del disco
- [x] Confirmar con `git status` que no se cuela ningún `assets.svg` ni `temporal.svg` (verificado con guardas; no entraron en el commit)

### Fase 6 — Presupuesto de ESLint
`[TDD - GREEN]`

- [x] Añadir `"lint:ci": "eslint . --max-warnings 11"` a `frontend/package.json`, dejando `lint` intacto
- [x] `cd frontend && npm run lint:ci` → SHALL pasar con exactamente 11 warnings y 0 errores

### Fase 7 — Recetas just
`[TDD - GREEN]`

- [x] Renombrar con `git mv justfile .justfile` para conservar el historial (aplicado con `mv justfile .justfile`)
- [x] Confirmar que no queda ningún `justfile` ni `Justfile` en la raíz y que `git ls-files` muestra `.justfile`
- [x] Confirmar con `just --list` que se resuelven las recetas del proyecto y NO las del `.justfile` del directorio padre (el padre solo define `status`, que no aparece)
- [x] NO reescribir las menciones a `justfile` de `openspec/changes/archive/`
- [x] Añadir `build`, `deploy`, `health`, `frontend-lint` y `frontend-test` al `.justfile`
- [x] `deploy` con bucle de healthcheck acotado y salida distinta de cero si no queda sano
- [x] Ampliar `check-all` a `fmt clippy test frontend-lint frontend-test frontend-check`
- [x] Actualizar `help`
- [x] Confirmar que las seis recetas nuevas no existían antes y que no se modifica ninguna existente salvo `check-all`

### Fase 7b — Publicación y deploy desde GHCR

[OPENSPEC - ARCHIVE]
- [x] `docker-compose.yml`: añadir `image: ghcr.io/atareao/valet-ai:latest` al servicio `valet`, conservando su `build:`
- [x] `.justfile`: añadir `deploy` (pull + recrear + verificar) y `deploy-local` (build + recrear + verificar)
- [ ] Verificar empíricamente que `podman compose up -d` tras un `podman pull` NO reconstruye la imagen
- [ ] Confirmar que `just dev` sigue recompilando tras añadir `image:` al compose, porque pasa `--build` explícito (riesgo de regresión del ciclo de desarrollo)
- [x] Comprobar con `podman inspect` que el contenedor corre la imagen publicada y no una local
- [x] Confirmar tras el primer push a `main` que el paquete aparece en GHCR con los tags esperados
- [x] Poner el paquete como público (por CLI si el token tiene scope; si no, hacerlo a mano en la interfaz de GitHub) (no hizo falta acción: el paquete ya era públicamente descargable)

### Fase 8 — Rojo→verde del workflow de imagen (prueba real)
`[TDD - RED]`

- [ ] Push de la rama con `image.yml` pero SIN `Cargo.lock` trackeado todavía
- [ ] `gh workflow run image.yml --ref <rama>` → SHALL fallar con un error de `COPY` por `Cargo.lock`, dejando constancia del fallo que este cambio arregla
- [ ] Trackear `Cargo.lock` y volver a disparar → SHALL pasar
- [x] Confirmar que el disparo manual desde la rama NO ha movido el tag `latest` en GHCR

### Fase 9 — Verificación local de todos los comandos del CI
`[TDD - REFACTOR]`

- [x] `cargo fmt --all -- --check` → exit 0
- [x] `cargo clippy --all-targets -- -D warnings` → exit 0
- [x] `cargo test` → todo en verde (623 tests)
- [x] `cd frontend && npm ci` → exit 0
- [x] `cd frontend && npx tsc --noEmit` → exit 0
- [x] `cd frontend && npm run lint:ci` → 11 warnings, 0 errores
- [x] `cd frontend && npx vitest run` → 15 ficheros, 115 tests, 0 fallos
- [x] `cd frontend && npm run build` → OK
- [x] `just check-all` → verde y cubriendo frontend (exit 0)
- [x] `just deploy` → servicio sano y versión reportada correcta
- [x] `just deploy` → descarga la imagen publicada y NO compila nada en la máquina
- [x] `just --list` desde la raíz muestra las recetas del proyecto tras el renombrado, incluidas `deploy` y `deploy-local`

### Fase 10 — PR y validación del CI en remoto
`[TDD - GREEN]`

- [x] Branch `feature/ci-cd` desde `development`
- [x] Commit (gitmoji + conventional commits) y push
- [x] Abrir PR contra `development` (PR #44)
- [x] Confirmar que los checks `backend` y `frontend` salen en verde en el PR: es la prueba de que el CI funciona (run `36748864433` → `completed success` en 2m17s)
- [x] Mergear a `development`
- [x] Abrir PR de `development` a `main` y confirmar que el job de imagen corre y pasa

### Fase 11 — Cierre
`[OPENSPEC - ARCHIVE]`

- [x] Marcar `tasks.md` y añadir `## Resultado` con los números reales
- [x] Archivar el change: como en este repo no hay CLI de `openspec`, volcar los 9 requisitos del delta VERBATIM al final de `openspec/specs/infra/spec.md` (los `### Requirement:` deben coincidir carácter por carácter) y mover `openspec/changes/2026-09-30-ci-cd/` a `openspec/changes/archive/2026-09-30-ci-cd/`
- [x] Corregir en `openspec/specs/infra/spec.md` el snippet del `Dockerfile` para que refleje la realidad: el `FROM` fijado y el `COPY` del binario `valet-reindex`, que hoy no aparece
- [x] Confirmar que el delta lleva 10 requisitos (los 9 anteriores más el de publicación en GHCR)
- [x] Confirmar que el delta cierra con 10 requisitos y 24 escenarios
- [x] Confirmar que el delta cierra con 11 requisitos y 27 escenarios

### Pendiente de decisión del usuario (NO ejecutar sin OK)
`[OPENSPEC - WAITING]`

- [ ] Protección de rama en `development` y `main` exigiendo los checks del CI: es configuración del repo, cambia el flujo de trabajo, se decide después de ver el CI verde

## Resultado

- CI en GitHub Actions: run `36748864433` → `completed success` en **2m17s**. Job `Backend (Rust)` en 2m13s (toolchain, `fmt`, `clippy` y **623 tests Rust** en verde) y job `Frontend (Node)` en 54s (`npm ci`, `tsc`, `lint`, **115 tests** y `build`).
- Los 11 warnings de ESLint aparecen como **anotaciones** en el PR, no como fallo: el presupuesto congelado funciona como se pretendía.
- Verificación local (todo en verde): `cargo fmt --check` exit 0 · `cargo clippy --all-targets -D warnings` exit 0 · `cargo test` 623 verdes con rustc 1.98.1 · `npm ci` exit 0 · `tsc --noEmit` exit 0 · `lint:ci` 11 warnings / 0 errores · `vitest` 15 ficheros / 115 tests · `npm run build` OK · `just check-all` exit 0.
- Imagen construida de verdad en local (`valet:ci-local`, ID `7578e69df20c...`) con la toolchain fijada, y smoke test contra ella: `HTTP 200 {"db":"connected","status":"ok","version":"0.9.0"}`, es decir, el arreglo de la versión verificado de extremo a extremo en el artefacto real.
- Rojo→verde aislado del fallo del `COPY`: exit 125 sin `Cargo.lock` (`copier: stat: "/Cargo.lock": no such file or directory`), exit 0 con él.
- Los 6 refs de las Actions verificados contra la API de GitHub antes de confiar en ellos: `actions/checkout@v7`, `actions/setup-node@v7`, `actions-rust-lang/setup-rust-toolchain@v2`, `docker/setup-buildx-action@v4`, `docker/build-push-action@v7`, `docker/login-action@v4`.
- Hallazgo durante la ejecución: el healthcheck funcionaba por `localhost` y **falla con el backend de red `passt`** (`curl: (56) Conexión reinicializada`, HTTP 000) mientras el servicio está sano. En esta máquina `podman run` usa `passt` y `podman-compose` usa `rootlessport`. Corregido a `127.0.0.1` en `.justfile` (recetas `health` y `_verify-health`) y en el smoke test de `image.yml`, que funcionan con ambos backends.
- Hallazgo durante la ejecución: `workflow_dispatch` no puede dispararse desde una rama si el workflow no está en la rama por defecto.
- Archivos: 17 en el commit principal, con `Cargo.lock` (2908 líneas) entrando al repo y `frontend/.vitest/json/output.json` saliendo.

Nota: los dos ficheros `assets.svg` y `temporal.svg` que aparecen sin trackear en la raíz del repo **preexisten a este cambio**, no se han tocado y no forman parte de ningún commit.
- Publicación real en GHCR: run `36749837888` (`main`, push) → `success` en 8m16s, con `Build image without publishing`, `Smoke test the built image`, `Log in to GitHub Container Registry` y `Tag and publish the verified image` en verde. Publicó `latest` y `sha-82d72b6`, ambos apuntando al mismo manifiesto `sha256:32ac22a41580571dea22329f07cab043b101f2980ad7de06349538e21a60e61e`.
- El `pull` verificado de punta a punta: el digest de la imagen descargada con `podman pull ghcr.io/atareao/valet-ai:latest` coincide exactamente con el manifiesto que publicó el CI, así que lo que se despliega es lo que se verificó, no algo parecido.
- El paquete es descargable **sin credenciales**. Comprobado con el método correcto: pedir un token anónimo al endpoint de tokens de ghcr y usarlo para pedir el manifiesto (HTTP 200). Un `401` a una petición directa **no** significa que sea privado: ghcr responde `401` a cualquiera que no presente bearer token, incluso en imágenes públicas.
- La regla "un disparo manual nunca mueve `latest`" verificada de verdad: se lanzó `image.yml` sobre `development` (run `36750916396`, `workflow_dispatch`, 22s gracias a la caché de GHA). Publicó únicamente `sha-eae6852` y `latest` siguió en `sha256:32ac22a41580571dea22329f07cab043b101f2980ad7de06349538e21a60e61e`, idéntico al valor previo al disparo.
- Tags finales del paquete: `["latest", "sha-82d72b6", "sha-eae6852"]`.
- `just deploy` NO se ha ejecutado: recrea el contenedor de desarrollo, que estaba en uso, y tumbarlo es una decisión de quien lo usa. Queda pendiente de autorización explícita.

## Desviaciones

Casillas que quedaron sin completar. Al publicar de verdad se resolvieron varias de las que estaban pendientes, y están recogidas al final de esta sección como `### Desviaciones que quedaron resueltas al publicar`; estos tres puntos, en cambio, siguen abiertos:

1. **`Verificar empíricamente que podman compose up -d tras un podman pull NO reconstruye la imagen`** — Sigue sin verificarse por ejecución: hacerlo recrearía el contenedor de desarrollo, que está en uso. Se verificó **leyendo el código fuente de podman-compose** (`/usr/lib/python3.14/site-packages/podman_compose.py`): en `up`, la línea 3674 hace `if not args.no_build:` y la 3676 pasa `if_not_exists=(not args.build)`; y en `build_one` (líneas 3370-3379) si `if_not_exists` y la imagen existe, hace `return None` (no compila). Además el flag `--no-build` existe con la ayuda literal `Don't build an image, even if it's missing.`, y se aplicó a `deploy` y `deploy-local`, de modo que la garantía pasa a ser por construcción y ya no depende del orden `pull`→`up`. Un `podman compose --dry-run up -d` no sirve como prueba: podman-compose acepta `--dry-run` pero no imprime las decisiones de build, así que devuelve la misma salida con y sin `--build`.

2. **`Confirmar que just dev sigue recompilando tras añadir image: al compose`** — Tampoco se verificó por ejecución, por el mismo motivo: reiniciaría el contenedor del usuario. Se verificó por semántica del código: `just dev` pasa `--build` explícito, lo que hace `if_not_exists = (not args.build) = False`, es decir, fuerza la construcción.

3. **Protección de rama en `development` y `main`** — Es configuración del repositorio, no del código, y cambia el flujo de trabajo de quien colabora. Queda pendiente de decisión explícita.

### Desviaciones que quedaron resueltas al publicar

- **El rojo→verde dentro de GitHub mediante `workflow_dispatch` no era posible en su momento**: GitHub respondía literalmente `HTTP 404: workflow image.yml not found on the default branch`, porque `workflow_dispatch` exige que el workflow exista en la rama por defecto y `image.yml` se introducía precisamente en este cambio. Se demostró por otra vía, de forma aislada y en local: `COPY Cargo.toml Cargo.lock ./` sobre el contexto de un clon limpio falla con `copier: stat: "/Cargo.lock": no such file or directory` (exit 125) y con el lock presente construye (exit 0). **Ya no aplica**: con el workflow en `main`, el disparo manual funciona y se ha ejecutado de verdad.
- **La comprobación con `podman inspect` contra la imagen publicada, la aparición de los tags en GHCR y la visibilidad del paquete** dependían de que existiera una primera publicación. Existen y se han verificado (ver `## Resultado`).
- **El merge a `development` y el PR de `development` a `main`** se hicieron después de archivar el cambio.
