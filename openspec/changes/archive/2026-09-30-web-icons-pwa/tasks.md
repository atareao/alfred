# Tasks

## 1. Iconos web y manifest PWA (RED → GREEN)

- [x] 1.1 Añadir `frontend/src/test/pwa-assets.test.ts` que verifique:
  - `frontend/index.html` contiene `rel="icon"` apuntando a `favicon.ico`, `rel="icon"` a `favicon-32x32.png`, `rel="apple-touch-icon"` a `apple-touch-icon.png` y `rel="manifest"` a `manifest.webmanifest`.
  - `frontend/index.html` NO contiene `vite.svg`.
  - `frontend/public/manifest.webmanifest` existe, es JSON válido y contiene `name`, `short_name`, `start_url`, `display`, `theme_color`, `background_color` e `icons`.
  - `icons` incluye entradas `192x192` y `512x512` de tipo `image/png`, y al menos una con `purpose: "maskable"`.
  - `frontend/public/` contiene `favicon.ico`, `favicon-16x16.png`, `favicon-32x32.png`, `favicon-48x48.png`, `apple-touch-icon.png`, `icon-192.png` e `icon-512.png`.
  Nota: se usan imports `?raw` de Vite e `import.meta.glob` (no `node:fs`) para no añadir `@types/node`. Ejecutar `cd frontend && npx vitest run src/test/pwa-assets.test.ts` y confirmar RED.
- [x] 1.2 Mover `assets/web/*` → `frontend/public/` (7 archivos) con `mv` (los assets estaban sin trackear). Ejecutar el test y confirmar que la parte de existencia de archivos pasa.
- [x] 1.3 Crear `frontend/public/manifest.webmanifest` con `name: "Valet"`, `short_name: "Valet"`, `start_url: "./"`, `display: "standalone"`, `theme_color: "#1677ff"`, `background_color: "#000000"` e `icons` (`./icon-192.png` `192x192` `any`; `./icon-512.png` `512x512` `any` + `maskable`). Ejecutar el test y confirmar GREEN.
- [x] 1.4 Actualizar `frontend/index.html`: eliminar `/vite.svg` y añadir los `link` a `./favicon.ico` (`sizes="any"`), `./favicon-32x32.png` (`32x32`), `./favicon-16x16.png` (`16x16`), `./apple-touch-icon.png` y `./manifest.webmanifest`; añadir `<meta name="theme-color" content="#1677ff">`. Ejecutar `cd frontend && npx vitest run src/test/pwa-assets.test.ts` y confirmar GREEN.

## 2. Generador de iconos

- [x] 2.1 Actualizar `assets/generate_icons_from_svg.fish` para que los iconos web se generen en `frontend/public/` (en lugar de `assets/web/`), manteniendo linux/android/ios en `assets/`. Verificar con `fish -n assets/generate_icons_from_svg.fish` (sintaxis) y revisar que no queden referencias a `assets/web`.

## 3. Build y sincronización de `static/`

- [x] 3.1 Ejecutar `cd frontend && npm run build` y confirmar que `frontend/dist/` contiene los 7 iconos y `manifest.webmanifest` en su raíz, y que `frontend/dist/index.html` referencia `manifest.webmanifest` (sin `vite.svg`).
- [x] 3.2 Sincronizar `static/` con el nuevo build (`rm -rf static && cp -r frontend/dist static`) para que `cargo run` sirva los iconos y el manifest.

## 4. Tests de servido backend (RED → GREEN)

- [x] 4.1 Añadir a `tests/api/frontend_serving.rs` (target `frontend-serving`) tests que verifiquen que el backend sirve los nuevos assets desde `static/`:
  - `GET /favicon.ico` → 200 y `Content-Type` contiene `image/x-icon` (o `image/vnd.microsoft.icon`), y el cuerpo NO es HTML.
  - `GET /icon-512.png` → 200 y `Content-Type` contiene `image/png`.
  - `GET /manifest.webmanifest` → 200 y `Content-Type` contiene `manifest+json` o `json`.
  Ejecutar `cargo test --test frontend-serving` y confirmar GREEN (los assets ya están en `static/` tras la tarea 3.2).

## 5. Verificación final

- [x] 5.1 Ejecutar `cd frontend && npx tsc --noEmit && npx vitest run && npm run lint`; confirmar todo verde.
- [x] 5.2 Ejecutar `cargo fmt --check && cargo clippy -- -D warnings && cargo test`; confirmar todo verde.
- [x] 5.3 Ejecutar `openspec validate web-icons-pwa --strict`; confirmar válido.
