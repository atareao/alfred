# Avatares en el chat: logo de Valet y avatar del usuario

## Intento
Dar identidad visual propia a las burbujas del chat:
1. Los mensajes del **asistente** pasan a mostrar el logotipo de Valet (SVG) en
   lugar del icono genérico `RobotOutlined` de Ant Design.
2. Los mensajes del **usuario** pasan a mostrar el avatar configurado en el perfil
   (`avatar_url`, pestaña *Perfil* de Ajustes) en lugar del icono genérico
   `UserOutlined`, con degradación a `UserOutlined` cuando no haya avatar
   configurado.

Además, el perfil pasa a obtenerse una sola vez mediante un contexto React
compartido (`ProfileProvider`), para que Ajustes y chat no mantengan copias
desincronizadas del perfil.

## Alcance
Solo frontend — no cambia backend, API, base de datos ni migraciones.
El campo `profiles.avatar_url` ya existe de punta a punta (migración
`20260925000001_initial.sql`, modelo `src/models/profile.rs`, repo
`src/db/repos/profiles.rs`, endpoints `GET/PUT /api/profile`, tipos TS en
`frontend/src/types/index.ts` y campo "Avatar URL" en `SettingsDialog`); solo
falta conectarlo al chat.

Fuera de alcance: cabecera, vista previa del avatar en Ajustes, y eliminar la
prop `settings` (hoy sin usar) de `ChatView`.

### Archivos a crear
- `frontend/src/assets/valet-icon.svg` — logotipo de Valet como SVG vectorial,
  derivado de `assets/icono.svg`, sin metadatos de Inkscape (namespaces
  `sodipodi`/`inkscape`) y con `viewBox` válido.
- `frontend/src/contexts/ProfileContext.ts` — `ProfileContextValue` (interface), `ProfileContext` (const) y el hook `useProfileContext()`, sin componentes ni JSX.
- `frontend/src/contexts/ProfileProvider.tsx` — el componente `ProfileProvider`, que reutiliza el hook `useProfile` y provee el contexto.

### Archivos a modificar
- `frontend/src/App.tsx` — envolver la app en `<ProfileProvider>`.
- `frontend/src/components/SettingsDialog.tsx` — consumir `useProfileContext()`
  en lugar de `useProfile()` directo.
- `frontend/src/components/AppLayout.tsx` — leer el perfil del contexto y pasar
  `userAvatarUrl` a `ChatView`.
- `frontend/src/components/ChatView.tsx` — aceptar `userAvatarUrl` y reenviarlo a
  cada `MessageBubble`.
- `frontend/src/components/MessageBubble.tsx` — avatar de Valet para `assistant`;
  avatar del perfil (o `UserOutlined`) para `user`; eliminar el import de
  `RobotOutlined`.
- `frontend/src/test/MessageBubble.test.tsx` — tests de ambos avatares.
- `frontend/src/components/AppLayout.test.tsx` y
  `frontend/src/test/SettingsDialog.test.tsx` — adaptar el mock del perfil al
  nuevo contexto.

### Archivos a eliminar
- Ninguno

### Nota de implementación
El contexto y el provider se separan en dos ficheros (`.ts` para el contexto y el hook,
`.tsx` para el componente) porque exportar un componente y código no-componente desde el
mismo módulo rompe el Fast Refresh de Vite y dispara el warning
`react-refresh/only-export-components` de ESLint. No se ha usado ningún `eslint-disable`.

## Impacto
- Mensajes del asistente (incluido el streaming, `id === "streaming"`): logo de Valet.
- Mensajes del usuario: avatar del perfil si existe; `UserOutlined` si no.
- Roles `system` y `tool`: sin cambios (`InfoCircleOutlined`, `CodeOutlined`).
- Una sola petición `GET /api/profile` en toda la app (antes: una por `SettingsDialog`).
- Al guardar un avatar nuevo en Ajustes, el chat lo refleja sin recargar la página.
- Sin dependencias nuevas: el SVG se importa como asset de Vite, ya soportado por
  `frontend/src/vite-env.d.ts` (`/// <reference types="vite/client" />`).
- Sin cambios en backend, API, migraciones, Dockerfile ni en el build de producción
  (el build de Vite emite `dist`, que el Dockerfile copia a `static/` para `ServeDir`).

## Riesgos
- `AppLayout.test.tsx` y `test/SettingsDialog.test.tsx` ya mockean
  `../hooks/useProfile` (el primero sin que `AppLayout` lo usara todavía). Al
  introducir el contexto, esos mocks deben adaptarse o los tests fallan.
- `frontend/tsconfig.json` tiene `noUnusedLocals: true`: si no se elimina el import
  de `RobotOutlined`, `npx tsc --noEmit` falla.
- El test existente "NO muestra timestamp para mensajes streaming" comprueba que el
  `textContent` no contenga `:`; el `alt` de los avatares NO debe introducir `:`.
