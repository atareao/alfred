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