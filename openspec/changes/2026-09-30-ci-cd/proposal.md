# CI en GitHub Actions y deploy local verificable

## Intento

Hoy el único control de calidad del repositorio es la memoria del autor. No existe
`.github/` ni ningún workflow: cada verificación depende de que alguien se acuerde de
ejecutarla a mano, y nada impide que un cambio roto llegue a `development` o a `main`.

Además hay un fallo latente demostrable, no una hipótesis: **el build de la imagen no
puede funcionar desde un clon limpio** porque `Cargo.lock` no está en git.

| Hecho | Evidencia |
|---|---|
| `Cargo.lock` no está trackeado | `git ls-files --error-unmatch Cargo.lock` → "no concordó con ningún archivo conocido por git" |
| Un clon limpio lo confirma | `git clone --depth 1` no produce `Cargo.lock` |
| El `Dockerfile` lo requiere | `COPY Cargo.toml Cargo.lock ./` en la línea 20 |

El build funciona hoy en la máquina del autor únicamente porque el fichero existe en
disco sin trackear. Cualquier otra máquina o un checkout limpio falla.

A este fallo se suma una mentira verificable: `/api/health` informa
`"version": "0.1.0"` mientras `Cargo.toml` declara `version = "0.9.0"`. El literal está
hardcodeado en `src/lib.rs:321`, y `tests/api/health.rs` tiene un test
(`test_health_body_version`) que afirma el valor equivocado, congelando el error y
bloqueando su arreglo.

Además, la imagen de la aplicación pasa a publicarse en GitHub Container Registry. El
despliegue deja de depender de compilar en la máquina: se descarga la imagen publicada,
de modo que el artefacto que se despliega es exactamente el mismo que el CI construyó y
verificó.

## Alcance

CI en GitHub Actions más recetas `just`, y los arreglos que desbloquean ese trabajo.

### Archivos a crear

- `.github/workflows/ci.yml` — CI rápido de backend y frontend en cada PR.
- `.github/workflows/image.yml` — build de la imagen y smoke test, solo en `main` y manual.
- `rust-toolchain.toml` — toolchain Rust fijada y compartida por local, CI e imagen.
- `openspec/changes/2026-09-30-ci-cd/` — este change proposal.

### Archivos a modificar

- `.gitignore` — sacar `Cargo.lock`, arreglar la línea corrupta, ignorar `frontend/.vitest/`.
- `Cargo.lock` — pasa de no trackeado a trackeado.
- `Dockerfile` — fijar el tag del compilador Rust.
- `justfile` → `.justfile` — renombrado con `git mv` para conservar el historial, por
  consistencia con el resto de repositorios de la máquina.
- `.justfile` — añadir `build`, `deploy`, `deploy-local`, `health`, `frontend-lint` y
  `frontend-test`, y ampliar `check-all`.
- `src/lib.rs` — el handler de `/api/health` deja de usar un literal de versión.
- `tests/api/health.rs` — el test compara contra la versión del paquete.
- `frontend/package.json` — añadir `lint:ci` con el presupuesto de warnings.
- `docker-compose.yml` — declarar `image:` con la ruta de `ghcr.io/atareao/valet-ai`
  además del `build:` actual.

### Datos recuperados del repo (pasan a trackearse o a dejar de estarlo)

- `frontend/.vitest/json/output.json` — hoy trackeado (19 KB, contiene rutas de otro
  proyecto). Pasa a no trackeado y se añade su directorio a `.gitignore`.

## Diseño

### CI rápido en cada PR (`.github/workflows/ci.yml`)

- Disparadores: `pull_request` contra `development` y `main`; `push` a `development` y
  `main`; `concurrency` con `cancel-in-progress: true`.
- Job `backend`, en `ubuntu-latest`, con los pasos exactos:
  1. `actions/checkout@v7`
  2. `actions-rust-lang/setup-rust-toolchain@v2` sin input `toolchain` (para que lea
     `rust-toolchain.toml`; la caché viene activada por defecto)
  3. `cargo fmt --all -- --check`
  4. `cargo clippy --all-targets -- -D warnings`
  5. `cargo test`
- Job `frontend`, en `ubuntu-latest`, con `defaults.run.working-directory: frontend`:
  1. `actions/checkout@v7`
  2. `actions/setup-node@v7` con `node-version: 22`, `cache: npm` y
     `cache-dependency-path: frontend/package-lock.json`
  3. `npm ci`
  4. `npx tsc --noEmit`
  5. `npm run lint:ci`
  6. `npx vitest run`
  7. `npm run build`
- Los dos jobs corren en paralelo y no comparten estado.
- Nota: `cargo clippy` se ejecuta con `--all-targets` porque el `.justfile` actual usa
  `clippy` a secas y no lintea los targets de test; ya verificado en verde con
  `--all-targets`.

### Imagen: construir, verificar y publicar (`.github/workflows/image.yml`)

- Disparadores: `push` a `main`, `push` de un tag `v*` y `workflow_dispatch`. El disparo
  manual es deliberado: permite ejecutar el job sobre una rama para demostrar el
  rojo→verde. Precisamente por eso un disparo manual publica SOLO el tag `sha-<corto>`:
  mover `latest` desde una rama publicaría código no mergeado como si fuera estable.
- Pasos, en este orden exacto:
  1. `actions/checkout@v7`
  2. `docker/setup-buildx-action@v4`
  3. `docker/build-push-action@v7` con `push: false`, `load: true`, `tags: valet:ci`,
     `cache-from: type=gha` y `cache-to: type=gha,mode=max`. Se construye SIN publicar
     todavía.
  4. Smoke test: arrancar el contenedor con
     `docker run -d --name valet-ci -p 3000:3000 -e DATABASE_URL=/tmp/valet.db valet:ci`,
     sondear `http://localhost:3000/api/health` hasta obtener 200 con `db = connected`, y
     verificar que la `version` del JSON coincide con la de `Cargo.toml`. Si falla, volcar
     `docker logs valet-ci` y abortar el job.
  5. `docker/login-action@v4` contra `ghcr.io`, con usuario `${{ github.actor }}` y
     contraseña `${{ secrets.GITHUB_TOKEN }}`.
  6. Etiquetar y publicar la MISMA imagen ya verificada con `docker push`, según los tags
     de abajo.
- El paso de publicación es posterior al smoke test: una imagen que no arranca nunca llega
  al registro. Publicar una imagen que no arranca es peor que no publicar.
- Registro: `ghcr.io/atareao/valet-ai`. Está vinculado automáticamente al repo, así que
  basta el `GITHUB_TOKEN` del workflow: NO se crean PATs ni secretos nuevos.
- El job DEBE declarar `permissions: packages: write`: el permiso por defecto del token de
  Actions en este repo es `read` (verificado con la API), y esa es la única configuración
  necesaria.
- Tags de publicación, en tres casos: en un `push` a `main` se publican `:latest` y
  `:sha-<corto>` (7 caracteres del SHA); en un tag `vX.Y.Z` se publican `:vX.Y.Z`, `:X.Y` y
  `:latest`; en un `workflow_dispatch` se publica únicamente `:sha-<corto>` y `latest` NO
  se mueve.
- Solo arquitectura `amd64`: la máquina del autor es `x86_64` (verificado) y añadir arm64
  con emulación encarecería enormemente el build de Rust.
- Visibilidad pública, para que el deploy local no necesite `podman login`. La imagen no
  contiene secretos: las claves de API entran por variables de entorno en tiempo de
  ejecución.
- `AUTH_ENABLED` no hace falta: su valor por defecto ya es `false` (`src/config.rs:80-82`).
- `DATABASE_URL=/tmp/valet.db` es deliberado: `create_if_missing(true)` crea el fichero
  pero no el directorio padre, y `/tmp` siempre existe y es escribible.

### Toolchain fijada (`rust-toolchain.toml`)

- `channel = "1.98.1"` y `components = ["rustfmt", "clippy"]`.
- Motivo: hoy local usa rustc 1.97.0 y la imagen 1.98.1; son dos compiladores distintos
  para el mismo código. Se elige 1.98.1 porque es el que ya compila la imagen en
  producción, así que está probado.
- El `Dockerfile` pasa de `docker.io/library/rust:alpine3.21` a
  `docker.io/library/rust:1.98.1-alpine3.21` (tag existe, verificado). Un tag móvil en
  el compilador significa que la toolchain cambia sin que cambie tu código.
- CONSECUENCIA que hay que advertir explícitamente: al aparecer `rust-toolchain.toml`,
  rustup instalará 1.98.1 en la máquina del autor en el siguiente comando de cargo. Es
  un cambio real en su entorno local.

### Presupuesto de warnings de ESLint

- Se añade `"lint:ci": "eslint . --max-warnings 11"` a `frontend/package.json`. Se
  conserva `lint` intacto.
- El 11 es el recuento actual congelado: convierte la deuda en un techo que no puede
  crecer.
- LÍMITE HONESTO: `--max-warnings` compara el total, no por regla. Si alguien borra un
  fichero que aportaba warnings, el presupuesto se relaja en esa cantidad. Se acepta por
  simplicidad; la alternativa (baseline por regla) es desproporcionada hoy. El número
  debe bajar, nunca subir.

### Deploy local (`just deploy`)

- **Ninguna de estas recetas existe hoy.** Las recetas actuales del `.justfile` son
  `check-all`, `check-spec`, `clean`, `clippy`, `dev`, `dev-docker`, `fmt`,
  `frontend-check`, `help` y `test`: no hay `build`, ni `deploy`, ni `deploy-local`, ni
  `health`, ni `frontend-lint`, ni `frontend-test`. Hoy el servicio se levanta con
  `just dev`, que es la única ruta existente y **recompila siempre** porque pasa `--build`
  explícito. Este cambio crea las recetas que faltan y **no modifica ninguna existente
  salvo `check-all`**.
- `just build` → `podman compose build`: construye la imagen en local.
- `just deploy` → `podman pull ghcr.io/atareao/valet-ai:latest`, recrear el servicio y
  verificar. Es el primer deploy que no compila: descarga una imagen ya construida y
  verificada en lugar de compilar Rust y el frontend en la máquina. NO pasa `--build`, que
  es precisamente lo que lo diferencia de `dev`.
- `just deploy-local` → construye la imagen en local y después hace lo mismo que `deploy`
  (recrear y verificar). Sirve para probar cambios del working tree.
- `docker-compose.yml` gana un `image: ghcr.io/atareao/valet-ai:latest` en el servicio
  `valet`, además del `build:` que ya tiene, para que el deploy pueda usar la imagen
  publicada sin recompilar. El `build:` se conserva para desarrollo local.
- El deploy comprueba que el contenedor que queda corriendo usa la imagen publicada y no
  una construida en local (inspeccionando el contenedor con `podman`).
- Ambas recetas de deploy esperan a que `/api/health` responda `status = ok` y
  `db = connected` con un bucle acotado, y fallan con código de salida distinto de cero si
  no queda sano. Al terminar, informan de la versión que reporta el servicio.
- `just health` → consulta puntual de `/api/health`.
- Se añaden `frontend-lint` (usa `npm run lint:ci`) y `frontend-test` (`npx vitest run`).
- `check-all` pasa a `fmt clippy test frontend-lint frontend-test frontend-check`: hoy NO
  ejecuta los 115 tests del frontend ni ESLint, así que la verificación local no cubría
  lo mismo que la CI.
- El `help` se actualiza con las recetas nuevas.
- NO se toca `docker-compose.prod.yml`.
- `just dev` NO cambia de comportamiento al añadir `image:` al compose, porque pasa
  `--build` explícito y por tanto sigue recompilando. Es la ruta de desarrollo y debe seguir
  siéndolo; el deploy sin compilar es `deploy`. Aun así, hay que verificarlo al ejecutar y
  no darlo por hecho: si `dev` dejara de reconstruir, rompería el ciclo de desarrollo.

### Renombrado de `justfile` a `.justfile`

- El fichero pasa de `justfile` a `.justfile` con `git mv`, para conservar el historial.
- Motivo, verificado en la máquina: **este repo es el único de todos** que usa `justfile`
  sin punto. Usan `.justfile` los repos `aqui`, `notas`, `notas2`, `podcasts`, `rust`,
  `Vídeos` y `workspace` bajo `/data`, además de `podman-quadlets/openclaw` y
  `podman-quadlets/voicebox`. La inconsistencia es real y el renombrado la cierra.
- `just` acepta ambos nombres y los resuelve en el orden `.justfile` → `justfile` →
  `Justfile`, así que el renombrado no cambia el comportamiento: solo el nombre.
- Detalle que hay que verificar y NO dar por hecho: existe `/data/rust/.justfile` en el
  directorio **padre** del proyecto, y `just` busca hacia arriba. Tras el renombrado hay que
  confirmar con `just --list` que se resuelven las recetas del proyecto y NO las del padre.
- El renombrado no rompe ninguna referencia: ni el `Dockerfile` ni `.dockerignore`
  mencionan el fichero. Lo único que dice "justfile" en el repo son referencias dentro de
  `openspec/changes/archive/` (`2026-09-23-f1-scaffolding`), que son registro histórico y
  NO se reescriben.

### Arreglo de la versión en /api/health

- `src/lib.rs` pasa a usar `env!("CARGO_PKG_VERSION")` en lugar del literal `"0.1.0"`.
- `tests/api/health.rs` pasa a afirmar `env!("CARGO_PKG_VERSION")`. El test actual
  congela el valor erróneo, así que primero se corrige el test (rojo) y después el
  handler (verde).

### Higiene de .gitignore

- Quitar `Cargo.lock`.
- Arreglar la línea corrupta `*.db-shm*.tsbuildinfo` separándola en dos reglas.
- Añadir `frontend/.vitest/` y sacar del índice `frontend/.vitest/json/output.json`.

## Impacto

- A partir de ahora, un PR que rompa formato, clippy, tests, tipos, lint o el build del
  frontend se ve en rojo antes de llegar a `development`.
- El build de la imagen deja de depender de un fichero que solo existe en un disco.
- Los builds pasan a ser reproducibles: hay lock y toolchain fijados.
- El contenedor se puede construir desde un clon limpio.
- El deploy deja de ser un comando a ciegas: se verifica el estado real del servicio.
- `/api/health` deja de informar una versión falsa.
- Coste: `ubuntu-latest` en repo público (minutos gratuitos). El job de imagen solo corre
  en `main` y en disparo manual.
- La imagen queda publicada y versionada en GHCR con tu nombre, con tag por SHA y por
  release: deja de existir solo en el disco de una máquina.
- El deploy pasa de compilar Rust y el frontend (~10 minutos) a descargar una imagen ya
  construida y verificada: segundos.
- El artefacto que se despliega es exactamente el que pasó el smoke test del CI, no una
  build distinta hecha en local.
- Cualquier otra máquina puede tirar de la misma imagen sin clonar el repo ni compilar.
- El fichero de recetas deja de ser el único de la máquina con un nombre distinto.

## Fuera de alcance (evaluado y descartado)

- Runner self-hosted y deploy por SSH: descartados porque el equipo es rootless podman
  sin ingreso público ni runner. La API confirma 0 runners.
- `docker-compose.prod.yml`: está roto (referencia `Dockerfile.backend` y
  `Dockerfile.frontend`, que no existen) pero es trabajo inacabado del autor, no un
  artefacto de este cambio. Se documenta y no se toca. Borrarlo podría ser mejor, pero eso
  no es asunto de este cambio.
- Arreglar los 11 warnings de ESLint: el usuario eligió congelarlos, no limpiarlos.
- Fijar la imagen de Node (`node:22-alpine`): sigue siendo un tag móvil, pero dentro de la
  misma minor y con riesgo de ruptura bajo. Se acepta.
- `cargo test --all-features`: el feature `vec0` (sqlite-vec) no lo usa ningún fichero del
  repo; activarlo no añadiría cobertura.
- Corrección de otras derivas de la spec `infra` (por ejemplo `reqwest 0.12 / rustls-tls`
  cuando el `Cargo.toml` real dice `reqwest 0.13` con `["json","rustls","stream"]`): es
  deriva previa y no relacionada.
- `frontend/.vitest/json/output.json`: se saca del índice pero NO se investiga cómo llegó
  ahí.
- **Auto-update automático y Quadlet:** descartado por decisión explícita del usuario ("por
  el momento no quiero hacer nada con quadlets"). La precisión técnica importa, porque la
  justificación fácil es incorrecta: `podman auto-update` no actualiza contenedores,
  actualiza **unidades systemd** — la documentación de `podman-auto-update(1)` dice que el
  contenedor *"must run inside a systemd unit"* y que la actualización se aplica *"by
  restarting the systemd units they run in"*. Un contenedor creado por podman-compose no es
  una unidad systemd, así que auto-update no tiene nada que reiniciar. Quadlet es
  simplemente la forma moderna de generar esa unidad (`podman generate systemd` está
  desaconsejada), de ahí que auto-update y Quadlet vayan juntos. Pero **no hace falta
  Quadlet para tener CD desatendido**: con podman-compose basta un timer de systemd que
  ejecute `just deploy`. Esa es la ruta barata y coherente con el stack actual, y queda
  documentada aquí, NO implementada: reiniciar Valet sin avisar puede cortar una
  conversación a medias, y este cambio ya cubre CI, publicación y deploy verificado.
- **Multi-arquitectura (`arm64`):** descartado. La máquina es `x86_64` (verificado) y añadir
  arm64 con emulación encarecería enormemente el build de Rust. La publicación es solo
  `amd64`.
- **Firma de imágenes (`cosign`) y escaneo de vulnerabilidades:** no se abordan en este
  cambio.
- Reescribir las menciones históricas a `justfile` que viven en
  `openspec/changes/archive/`: son el registro de lo que se decidió en su momento y no se
  tocan. El renombrado afecta al nombre del fichero, no a la historia.

## Decisión pendiente (no se ejecuta sin OK explícito del usuario)

- Protección de rama: exigir que los checks del CI estén en verde para poder mergear en
  `development` y `main`. Es un ajuste de configuración del repo, no código. Cambia el
  flujo de trabajo del usuario, así que queda fuera de la ejecución y se decide después de
  ver el CI funcionando.
- Poner el paquete de GHCR como público: el paquete nace privado por defecto aunque el repo
  sea público. La intención es dejarlo público, para que el deploy local no necesite
  `podman login ghcr.io` con un PAT. Si el token de la CLI no tiene scope `write:packages`,
  habrá que hacerlo a mano desde la interfaz de GitHub (Package settings → Change
  visibility).

## Verificación

- Cada comando del CI se replica a mano en local antes del PR (los 9), con sus resultados
  esperados.
- La validez del propio CI se comprueba abriendo el PR: los checks deben salir en verde.
- El workflow de imagen se prueba con `workflow_dispatch` sobre la rama ANTES de trackear
  `Cargo.lock`, para dejar constancia del fallo `COPY failed: Cargo.lock: not found`, y
  otra vez DESPUÉS, para dejar constancia del verde. Ese es el rojo→verde de este cambio.
- No hay `act` instalado, así que la prueba de los workflows es remota, no local.
- Se comprueba que la imagen aparece en GHCR con los tags esperados tras el primer push a
  `main`.
- Se comprueba que `podman pull ghcr.io/atareao/valet-ai:latest` funciona.
- Se comprueba que `just deploy` NO compila nada y deja el contenedor corriendo la imagen
  publicada.

## Riesgos

- Los workflows no se pueden probar en local: la primera ejecución real puede destapar
  errores de sintaxis YAML o de nombres de inputs. Se asume y se itera sobre el PR.
- El job de imagen es lento (compila Rust y el frontend dentro de Docker). Por eso solo
  corre en `main`.
- El presupuesto de ESLint es un techo móvil a la baja (ver el límite honesto arriba).
- El paquete de GHCR nace privado por defecto: si no se cambia la visibilidad, el deploy
  necesitará autenticación por PAT, que caduca y hay que mantener.
- Un disparo manual mal entendido podría parecer que "publica la rama como estable": no lo
  hace, porque solo publica el tag del SHA, pero conviene saberlo.
