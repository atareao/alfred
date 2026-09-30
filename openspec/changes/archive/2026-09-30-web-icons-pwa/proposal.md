# Proposal: Iconos web y PWA desde `frontend/public/`

## Why

El kit de iconos generado por `assets/generate_icons_from_svg.fish` vive en `assets/web/` (favicons, `apple-touch-icon`, `icon-192`, `icon-512`, `favicon.ico`), pero **ninguno se usa**: Vite no tiene directorio `public/`, `frontend/index.html` referencia `/vite.svg` (que no existe) y no hay `manifest.webmanifest`. El resultado es un favicon roto y una app no instalable como PWA, pese a que los iconos ya están generados.

## What Changes

- **Mover** `assets/web/*` → `frontend/public/` (directorio público de Vite). Vite copia su contenido a la raíz de `dist/`, y el backend lo sirve desde `static/`.
- **Actualizar** `frontend/index.html` para referenciar los iconos reales (`favicon.ico`, `favicon-32x32.png`, `favicon-16x16.png`, `apple-touch-icon.png`) y el manifest PWA, eliminando la referencia muerta a `/vite.svg`.
- **Crear** `frontend/public/manifest.webmanifest` (PWA) con `name`, `short_name`, `start_url`, `display: standalone`, `theme_color`, `background_color` e `icons` (`icon-192.png`, `icon-512.png`, uno de ellos `maskable`).
- **Actualizar** `assets/generate_icons_from_svg.fish` para que los iconos web se generen directamente en `frontend/public/` (los de linux/android/ios siguen en `assets/`).
- **Sincronizar** el `static/` versionado (build local servido por `cargo run`) reconstruyendo el frontend, para que el favicon y el manifest también funcionen en desarrollo sin Docker.

## Capabilities

### New Capabilities

<!-- Ninguna -->

### Modified Capabilities

- `frontend-serving`: el frontend SHALL publicar los iconos web y el manifest PWA desde `frontend/public/`, y `index.html` SHALL referenciarlos; el backend los sirve como assets estáticos.

## Impact

- **Assets movidos**: `assets/web/*` → `frontend/public/` (7 archivos: `favicon-16x16.png`, `favicon-32x32.png`, `favicon-48x48.png`, `favicon.ico`, `apple-touch-icon.png`, `icon-192.png`, `icon-512.png`).
- **Frontend**: `frontend/index.html` (nuevas etiquetas `link`), `frontend/public/manifest.webmanifest` (nuevo).
- **Tooling**: `assets/generate_icons_from_svg.fish` (ruta de salida web).
- **Build**: `frontend/dist/` (regenerado) y `static/` (sincronizado).
- **Sin cambios** en backend Rust, API, base de datos ni dependencias.
- **No se tocan** los iconos de `assets/linux/`, `assets/android/` ni `assets/ios/` (no hay proyectos nativos donde usarlos).
