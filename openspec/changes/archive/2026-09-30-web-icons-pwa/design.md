# Design

## Context

Ver `proposal.md - Why`. Restricciones relevantes:

- El frontend es Vite + React con `base: ''` (rutas relativas) y `publicDir` por defecto (`frontend/public/`), que **no existe**.
- El backend Axum sirve `static/` con fallback SPA (`ServeDir::new("static").fallback(ServeFile::new("static/index.html"))`).
- El `Dockerfile` construye el frontend (`npm run build`) y copia `frontend/dist` → `/app/static`.
- El `static/` versionado es un build antiguo (solo `index.html` + un JS con hash) que se usa al ejecutar `cargo run` sin Docker.

## Goals / Non-Goals

**Goals:**
- Que los iconos web ya generados se sirvan realmente y que la app sea instalable como PWA.
- Mantener el flujo de build existente (Vite → `dist` → `static`) sin tocar el backend.

**Non-Goals:**
- Empaquetado nativo (Android/iOS/Linux) ni service worker offline.
- Rediseñar el sistema de assets ni migrar a imports de TS.

## Decisions

### D1 — Usar `frontend/public/` (publicDir de Vite) en lugar de `static/` o imports TS

Vite copia `public/` tal cual a la raíz de `dist/`, sin hashing ni procesado. Es el mecanismo estándar para favicons/manifest y encaja con el `Dockerfile` actual (`dist` → `static`). Alternativas descartadas: importar los PNG desde TS (los hashea y complica las rutas del manifest) y escribir directamente en `static/` (rompe el flujo de build y el `dist` quedaría sin iconos).

### D2 — Rutas relativas en `index.html` y en el manifest

Con `base: ''`, el HTML construido usa rutas relativas (`./assets/...`). Para no depender de que Vite reescriba las referencias del `public/`, se usarán rutas relativas (`./favicon.ico`, `./manifest.webmanifest`) y en el manifest `start_url: "./"` e `icons` con `./icon-192.png`. Así funciona igual servido en `/` o en un subpath. Se verificará empíricamente el HTML resultante tras `npm run build`.

### D3 — Manifest con `display: standalone` y un icono `maskable`

`icon-192.png` como `any` y `icon-512.png` como `any` + `maskable` (reutilizando el mismo PNG). `theme_color` `#1677ff` (color primario del tema Ant Design) y `background_color` `#000000` (fondo del layout). Alternativa descartada: generar un PNG maskable dedicado (fuera de alcance; el icono ya tiene fondo).

### D4 — Sincronizar `static/` reconstruyendo el frontend

Para que `cargo run` (sin Docker) sirva los iconos y el manifest, se reconstruye `frontend/dist` y se copia a `static/`. El hash del bundle JS cambiará; es esperado. Alternativa descartada: dejar `static/` intacto (el favicon seguiría roto en dev local).

### D5 — Actualizar el generador para que escriba los iconos web en `frontend/public/`

`assets/generate_icons_from_svg.fish` seguirá generando linux/android/ios en `assets/`, pero los web pasarán a `frontend/public/`, manteniendo una única fuente de verdad y evitando que un futuro `generate` vuelva a crear `assets/web/`.

## Risks / Trade-offs

- [Vite no reescribe las rutas del `public/` con `base: ''`] → Se usan rutas relativas explícitas y se valida el `dist/index.html` tras el build.
- [`static/` versionado se desincroniza del `dist`] → Se sincroniza en esta tarea; a futuro convendría automatizarlo o dejar de versionarlo (fuera de alcance).
- [El manifest con `start_url: "./"` puede comportarse distinto según el navegador] → Se valida que el JSON sea correcto y que el backend lo sirva con el MIME adecuado.

## Migration Plan

1. Mover los iconos y crear el manifest.
2. Actualizar `index.html` y el generador.
3. `npm run build` y sincronizar `static/`.
4. Rollback: revertir el commit (los iconos vuelven a `assets/web/`); no hay migración de datos ni cambios de API.
