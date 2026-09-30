# Frontend Serving: Static assets and SPA fallback

## Purpose

Define cómo el backend Axum sirve los assets del frontend SPA compilados (HTML, JS, CSS) y maneja el fallback de rutas SPA, eliminando la necesidad de nginx o un contenedor frontend separado.

## Contratos

### Backend sirve assets estáticos compilados

El backend sirve los archivos estáticos del frontend (compilados por Vite) desde el directorio `static/`. La ruta base del bundle Vite es relativa (`base: ''`).

### Fallback SPA para client-side routing

El backend implementa un fallback SPA: cualquier ruta que no coincida con una ruta de API o un asset estático sirve `static/index.html`. Implementado con `ServeDir::new("static").fallback(ServeFile::new("static/index.html"))` en el router Axum.

### Vite base path relativo

El `vite.config.ts` configura `base: ''` (ruta relativa) para que los assets JS/CSS se resuelvan correctamente al servirse desde el mismo origen que la API.

### Modo desarrollo con Vite HMR

En desarrollo, Vite puede ejecutarse independientemente con su propio dev server (puerto 5173) y proxy inverso hacia el backend (puerto 3000), manteniendo HMR.

## Escenarios

### Petición GET / devuelve index.html
**Given** el frontend compilado en `static/index.html`  
**When** se solicita `GET /` al backend  
**Then** el backend responde 200 con `Content-Type: text/html`  
**And** el cuerpo contiene el HTML del SPA con los tags script y link apuntando a rutas relativas

### Ruta SPA /chat/abc-123 sirve index.html
**Given** el frontend compilado en `static/index.html`  
**When** se solicita `GET /chat/abc-123`  
**And** no existe el archivo `static/chat/abc-123`  
**And** no coincide con ninguna ruta de API  
**Then** el backend responde 200 con `Content-Type: text/html`  
**And** el cuerpo es el contenido de `static/index.html`

### API route /api/health no se ve afectada por el fallback
**Given** el router con rutas API y fallback SPA  
**When** se solicita `GET /api/health`  
**Then** el backend responde con JSON `{"status":"ok"}`, no con index.html

### Vite build produce assets con rutas relativas
**Given** `vite.config.ts` con `base: ''`  
**When** se ejecuta `npm run build` en el directorio frontend/  
**Then** el `index.html` generado contiene `src="./assets/index-<hash>.js"`  
**And** no hay referencias a rutas absolutas como `/assets/`

### Desarrollo con Vite proxy a backend
**Given** el frontend en modo desarrollo (puerto 5173)
**When** el frontend hace fetch a `/api/health`
**Then** Vite redirige la petición a `http://localhost:3000/api/health`
**And** HMR funciona correctamente (cambios en TSX se reflejan en tiempo real)

## Requirements

### Requirement: Frontend SHALL use configurable message page size

**Given** el hook `useMainChat`
**When** se carga la conversación inicial
**Then** SHALL usar el valor de `message_page_size` del setting en lugar de hardcodeado 50
**And** `loadMore` SHALL usar el mismo valor

#### Scenario: Carga inicial usa el setting
**Given** `message_page_size` = 25 en settings
**When** se monta `useMainChat`
**Then** llama `api.listMessages(conv.id, 25)`

#### Scenario: loadMore usa el mismo valor
**Given** `message_page_size` = 25 en settings
**When** se invoca `loadMore`
**Then** llama `api.listMessages(conv.id, 25, cursor)`

### Requirement: SettingsEditor SHALL include message_page_size field

**Given** el panel de ajustes
**When** está visible
**Then** SHALL mostrar un campo `message_page_size` con min=10, max=100, step=10
**And** el valor por defecto SHALL ser 50

#### Scenario: Campo message_page_size visible en ajustes
**Given** el panel de ajustes
**When** está visible
**Then** muestra un campo `message_page_size` con min=10, max=100, step=10
**And** el valor por defecto es 50

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
