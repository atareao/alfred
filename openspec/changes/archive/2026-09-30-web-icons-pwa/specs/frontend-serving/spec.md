# Spec Delta: frontend-serving

## ADDED Requirements

### Requirement: Web icons SHALL be published from the frontend public directory

Los iconos web SHALL residir en `frontend/public/` (directorio público de Vite), de modo que `npm run build` los copie a la raíz de `frontend/dist/` y el backend los sirva como assets estáticos.

**Given** el directorio `frontend/public/`
**When** se ejecuta `npm run build` en `frontend/`
**Then** `frontend/dist/` SHALL contener `favicon.ico`, `favicon-16x16.png`, `favicon-32x32.png`, `favicon-48x48.png`, `apple-touch-icon.png`, `icon-192.png` e `icon-512.png` en su raíz
**And** `assets/web/` SHALL NOT contener ya esos archivos

#### Scenario: El build copia los iconos a la raíz de dist
- **GIVEN** los iconos web en `frontend/public/`
- **WHEN** se ejecuta `npm run build`
- **THEN** `frontend/dist/favicon.ico`, `frontend/dist/icon-192.png` y `frontend/dist/icon-512.png` existen

#### Scenario: El backend sirve el favicon
- **GIVEN** el backend arrancado con `static/` sincronizado
- **WHEN** se solicita `GET /favicon.ico`
- **THEN** el backend responde 200 con `Content-Type: image/x-icon` (o `image/vnd.microsoft.icon`)
- **AND** no devuelve `index.html`

#### Scenario: El backend sirve el icono PWA de 512
- **GIVEN** el backend arrancado con `static/` sincronizado
- **WHEN** se solicita `GET /icon-512.png`
- **THEN** el backend responde 200 con `Content-Type: image/png`

### Requirement: index.html SHALL reference the web icons and the PWA manifest

`frontend/index.html` SHALL declarar los iconos web y el manifest PWA, y SHALL NOT referenciar `/vite.svg`.

**Given** `frontend/index.html`
**When** se inspecciona su `<head>`
**Then** SHALL contener `<link rel="icon" href="...favicon.ico" sizes="any">`
**And** SHALL contener `<link rel="icon" type="image/png" sizes="32x32" href="...favicon-32x32.png">`
**And** SHALL contener `<link rel="apple-touch-icon" href="...apple-touch-icon.png">`
**And** SHALL contener `<link rel="manifest" href="...manifest.webmanifest">`
**And** SHALL NOT contener `vite.svg`

#### Scenario: No queda referencia a vite.svg
- **GIVEN** `frontend/index.html`
- **WHEN** se busca la cadena `vite.svg`
- **THEN** no aparece

#### Scenario: El HTML construido referencia el manifest
- **GIVEN** `frontend/index.html` con el link al manifest
- **WHEN** se ejecuta `npm run build`
- **THEN** `frontend/dist/index.html` contiene una referencia a `manifest.webmanifest`

### Requirement: The app SHALL expose a PWA manifest

La aplicación SHALL exponer `frontend/public/manifest.webmanifest` con los metadatos mínimos de instalación y los iconos `icon-192.png` e `icon-512.png`.

**Given** `frontend/public/manifest.webmanifest`
**When** se parsea como JSON
**Then** SHALL contener `name`, `short_name`, `start_url`, `display` (`standalone`), `theme_color`, `background_color` e `icons`
**And** `icons` SHALL incluir una entrada de `192x192` y otra de `512x512` de tipo `image/png`
**And** al menos una entrada SHALL declarar `purpose: "maskable"`

#### Scenario: El manifest es JSON válido con los campos requeridos
- **GIVEN** `frontend/public/manifest.webmanifest`
- **WHEN** se parsea como JSON
- **THEN** `name`, `short_name`, `start_url`, `display`, `theme_color`, `background_color` e `icons` están presentes

#### Scenario: El manifest declara los dos tamaños de icono
- **GIVEN** el manifest PWA
- **WHEN** se inspecciona `icons`
- **THEN** existe una entrada con `sizes: "192x192"` y otra con `sizes: "512x512"`
- **AND** ambas tienen `type: "image/png"`

#### Scenario: El backend sirve el manifest
- **GIVEN** el backend arrancado con `static/` sincronizado
- **WHEN** se solicita `GET /manifest.webmanifest`
- **THEN** el backend responde 200 con `Content-Type: application/manifest+json` (o `application/json`)
