# Tasks — Avatares del chat: logo de Valet + avatar del usuario

## TDD checklist

### RED — Write failing tests
- [x] Scenario: mensaje `assistant` renderiza un `<img>` del logo de Valet y NO el icono robot
- [x] Scenario: mensaje `streaming` (`id === "streaming"`, rol `assistant`) usa el logo de Valet
- [x] Scenario: roles `system` y `tool` conservan su icono y no usan el logo de Valet
- [x] Scenario: usuario con `userAvatarUrl` renderiza la imagen y NO `UserOutlined`
- [x] Scenario: usuario sin `userAvatarUrl` degrada a `UserOutlined`
- [x] Scenario: `userAvatarUrl` no se usa en roles `assistant` / `system` / `tool`
- [x] Scenario: `ChatView` reenvía `userAvatarUrl` a las burbujas (incluido el mensaje de streaming)
- [x] Scenario: `ChatView` sin la prop `userAvatarUrl` no rompe el render
- [x] Scenario: `useProfileContext()` fuera del provider lanza error
- [x] Scenario: editar el avatar en Ajustes actualiza el contexto sin recargar
- [x] `cd frontend && npx vitest run` → los tests nuevos en ROJO, los legacy en VERDE

### GREEN — Minimal implementation
- [x] Crear `frontend/src/assets/valet-icon.svg` (sin metadatos de Inkscape, con `viewBox` válido)
- [x] Crear `frontend/src/contexts/ProfileContext.tsx` con `ProfileProvider` y `useProfileContext()`, reutilizando `useProfile`
- [x] `App.tsx`: envolver la app en `<ProfileProvider>`
- [x] `AppLayout.tsx`: leer el perfil del contexto y pasar `userAvatarUrl` a `<ChatView>`
- [x] `ChatView.tsx`: añadir la prop opcional `userAvatarUrl` y reenviarla a cada `MessageBubble`
- [x] `MessageBubble.tsx`: logo de Valet para `assistant`; `userAvatarUrl` (o `UserOutlined`) para `user`; `alt` accesible y tamaño ~24x24
- [x] `SettingsDialog.tsx`: consumir `useProfileContext()` en lugar de `useProfile()`
- [x] Adaptar los mocks de `AppLayout.test.tsx` y `test/SettingsDialog.test.tsx` al contexto
- [x] `npx vitest run` → 100% verde
- [x] `npx tsc --noEmit` → sin errores de tipo
- [x] Envolver en <ProfileProvider> los 4 tests legacy debug*.test.tsx que montan AppLayout (daño colateral)

### REFACTOR
- [x] Eliminar el import de `RobotOutlined` de `MessageBubble.tsx`
- [x] `npm run lint` → sin warnings
- [x] Re-ejecutar `npx vitest run` → sin regresiones
- [x] Verificación visual en fondo oscuro: logo de Valet nítido a 24x24 y avatar de usuario circular, alineados con la burbuja
- [x] Confirmar que no queda ninguna llamada duplicada a `GET /api/profile`
- [x] Separar el contexto (.ts) del provider (.tsx) para eliminar el warning react-refresh/only-export-components sin eslint-disable
- [x] Verificar que ninguna casilla queda sin marcar

## Resultado

- `npx vitest run` → 18 ficheros, 101 tests, 0 fallos.
- `npx tsc --noEmit` → exit 0.
- `npx eslint` → 0 errores; 11 warnings, todos preexistentes en el repo y ajenos a esta feature.
- Verificación visual del asset: `frontend/src/assets/valet-icon.svg` renderizado a PNG produce un
  bitmap idéntico (mismo MD5) al del logotipo original `assets/icono.svg`.
- Follow-ups NO incluidos en este cambio:
  - Degradación ante `avatar_url` roto (404/host caído): falta un `onError` que caiga a `UserOutlined`.
  - Limpieza de los 4 `debug*.test.tsx` (tests sin aserciones, deuda previa del repo).
  - Separar `isSaving` de `loading` en `useProfile` para que el contexto no exponga el loading de mutación.
  - `assets.svg` y `temporal.svg` sin trackear en la raíz del repo (ajenos a esta feature).
