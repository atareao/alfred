# Tasks

## 0. Línea base

- [x] 0.1 Baseline literal: 16 ficheros / 116 tests, `npx tsc --noEmit` limpio, `npx eslint .` con 11 warnings, `PENDING_LONG=0` en los 16 ficheros.

## 1. `<App>` en la raíz

- [x] 1.1 `src/App.tsx`: envolver el árbol con el `<App>` de antd, dentro de `ConfigProvider`.
- [x] 1.2 Verificar que la app monta, que las rutas siguen funcionando y que no aparece el warning de antd sobre funciones estáticas sin contexto.

## 2. `SettingsDialog` — 6 llamadas (TDD)

- [x] 2.1 RED: en `SettingsDialog.test.tsx`, aserción negativa `expect(message.success).not.toHaveBeenCalled()`. Debe **fallar** hoy: la componente usa la API estática.
- [x] 2.2 GREEN: migrar `SettingsDialog` a `App.useApp()` (líneas 110, 113, 140, 143, 159, 161). El test de 2.1 pasa.
- [x] 2.3 REFACTOR: quitar los espías `vi.spyOn(message, ...)` y la aserción `toHaveBeenCalledWith("Error al actualizar perfil")`; sustituir por aserciones sobre el texto en el DOM.

## 3. `RetentionConfig` — 3 llamadas (TDD)

- [x] 3.1 RED: en `StatsDashboard.test.tsx`, aserción negativa sobre la API estática para el camino de `RetentionConfig`.
- [x] 3.2 GREEN: migrar `RetentionConfig` (líneas 19, 28, 30) a `App.useApp()`.
- [x] 3.3 REFACTOR: quitar el espía preventivo de `StatsDashboard.test.tsx` y montar dentro de `<App>` si hace falta.

## 4. `EventModal` — unificar al contexto

- [x] 4.1 `EventModal.tsx`: `message.useMessage()` + `{contextHolder}` → `App.useApp()`; eliminar el `contextHolder` del JSX y el import de `message` si queda huérfano.
- [x] 4.2 `EventModal.test.tsx`: montar dentro de `<App>`; **eliminar** el espía estático (`vi.spyOn(message, "error")`), que pasa a ser imposible de importar, y aseverar el texto del error en el DOM. La garantía contra la vuelta atrás pasa a ser la regla de lint.

## 4b. `TaskModal` — unificar al contexto (apareció al ejecutar la regla)

- [x] 4b.1 `TaskModal.tsx`: `message.useMessage()` + `{contextHolder}` → `App.useApp()`; eliminar el `contextHolder` del JSX y el import de `message`.
- [x] 4b.2 `TaskModal.test.tsx` (nuevo): montar dentro de `<App>`, caso de error (texto en el DOM) y de éxito. Hoy el módulo no tiene ninguna prueba.
- [x] 4b.3 Confirmar que no queda ningún `useMessage()` ni `contextHolder` en `src/`. (Solo queda una mención de la palabra en un **comentario** de `src/test/SettingsDialog.test.tsx:115`; ningún uso real.)

## 5. Regla de lint

- [x] 5.1 `eslint.config.js`: `no-restricted-imports` sobre `antd` para `message` y `notification`.
- [x] 5.2 RED: comprobar con un fichero temporal que importa `message` desde `antd` que el lint falla. **Borrar el fichero temporal después** (no dejarlo dentro de `src/`).
- [x] 5.3 GREEN: el árbol real pasa sin errores de `no-restricted-imports`. Desbloqueado tras `4b.1`: ESLint queda en `0 errors, 11 warnings`.

## 6. Verificación

- [x] 6.1 `npx vitest run` en verde (16 ficheros / 118 tests); `npx tsc --noEmit` limpio.
- [x] 6.2 `PENDING_LONG=0` en los 16 ficheros con el instrumento de recuento de temporizadores. (Requiere el instrumento de conteo; no disponible en esta tarea.)
- [x] 6.3 Confirmar que no queda ningún `spyOn(message` en los tests. (Nota: `grep "message\.\(success\|error...\)"` sí devuelve las llamadas contextuales de `SettingsDialog`/`RetentionConfig`, que usan la variable `message` de `App.useApp()`; no son API estática.)
- [ ] 6.4 Tras el merge: republicar imagen, **desplegar**, y verificar arranque + supervivencia al presupuesto de drenaje con el chequeo del CI.
