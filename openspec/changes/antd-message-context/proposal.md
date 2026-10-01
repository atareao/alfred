# Proposal: API contextual de antd (antd-message-context)

## Why

antd ofrece dos formas de mostrar un aviso: la **estática** (`import { message } from "antd";` y luego
`message.success(...)`) y la **contextual** (`App.useApp()` o `message.useMessage()`).

La estática tiene un problema estructural: crea el nodo del aviso **fuera del árbol de React**. De ahí
salen dos consecuencias, una visible y otra que costó un CI en rojo:

1. **Su temporizador escapa al ciclo de vida de React.** Cada aviso programa un `setTimeout` de
   3000 ms que no se limpia al desmontar el componente. En jsdom eso se materializa como
   `ReferenceError: window is not defined` lanzado desde `rc-notification` cuando el temporizador
   dispara después del teardown del entorno de pruebas — **con las 115 pruebas en verde**. Ese fue el
   fallo intermitente que puso `main` en rojo y que se midió en 11 temporizadores de 3000 ms vivos al
   terminar la suite. Que no se reproduzca en local no lo hace falso: es una carrera entre el
   temporizador y el teardown.
2. **Ignora el `ConfigProvider`.** El aviso no hereda tema ni contexto, así que puede salirse del
   estilo del resto de la app.

Estado actual: **9 llamadas a la API estática** — `SettingsDialog.tsx` (líneas 110, 113, 140, 143,
159, 161) y `RetentionConfig.tsx` (19, 28, 30). `EventModal.tsx` ya usa la contextual
(`message.useMessage()` + `contextHolder`), pero con la variante *por componente*.

La mitigación que hay hoy es un parche en las pruebas: `SettingsDialog.test.tsx` y
`StatsDashboard.test.tsx` **espían y neutralizan** `message.success`/`message.error` para que no
programen temporizadores. Funciona, pero trata el síntoma y no la causa, y hay que repetirlo en cada
fichero de test que monte un componente que use la API estática. Es deuda que crece sola.

## What Changes

### 1. `<App>` de antd en la raíz

`src/App.tsx` envuelve el árbol con el componente `<App>` de antd, dentro de `ConfigProvider`, para
que `App.useApp()` esté disponible en cualquier descendiente.

### 2. Migrar las 9 llamadas estáticas

`SettingsDialog.tsx` y `RetentionConfig.tsx` pasan a obtener `message` de `App.useApp()`. Los textos
de los avisos **no cambian** ni en contenido ni en idioma.

### 3. Unificar `EventModal` y `TaskModal`

`message.useMessage()` + `{contextHolder}` → `App.useApp()`, eliminando el `contextHolder` del JSX.
Así queda **un solo idioma** en todo el frontend en vez de dos formas de hacer lo mismo.

`TaskModal` apareció al ejecutar la regla de lint: usa `message.useMessage()`, la variante por
componente. Es contextual y **no** tiene el problema del temporizador, pero mantiene vivo el import de
`message` y por tanto obligaba a dejar una excepción en la regla — lo que la habría degradado de
garantía a recomendación. Se unifica para poder prohibirlo sin excepciones. Hoy **no tiene ninguna
prueba**: se añade una mínima, porque cambiar producción sin cobertura es justo lo que este change no
debe hacer.

### 4. Una regla de lint que lo impide

`no-restricted-imports` sobre `antd` para `message` y `notification`: reintroducir la API estática
pasa a ser un error de lint, no un descuido que se cuela en una revisión.

### 5. Quitar el parche de las pruebas

Eliminar los espías neutralizadores de `SettingsDialog.test.tsx`, quitar el espía preventivo de
`StatsDashboard.test.tsx` y volver a aserciones sobre el DOM. `EventModal.test.tsx` pasa a montar el
componente dentro de `<App>`.

Al prohibir el lint la importación de `message`, el espía de `EventModal.test.tsx` deja de ser
siquiera escribible: para espiar la API estática hay que importarla, e importarla es un error. La
garantía contra la vuelta atrás pasa de ser una aserción en tiempo de ejecución a ser **estática**, y
el lint la aplica antes de que las pruebas lleguen a correr.

## Impact

- **Ficheros**: `src/App.tsx`, `SettingsDialog.tsx`, `RetentionConfig.tsx`, `EventModal.tsx`,
  `TaskModal.tsx`, `eslint.config.js`, `SettingsDialog.test.tsx`, `StatsDashboard.test.tsx`,
  `EventModal.test.tsx` y `TaskModal.test.tsx` (nuevo).
- **Comportamiento**: los avisos se siguen mostrando con los mismos textos; dejan de crear
  temporizadores que sobrevivan al desmontaje y pasan a heredar el contexto del `ConfigProvider`.
- **Pruebas**: se **elimina** el hack de los espías; el resultado es una suite más simple y sin
  aserciones sobre detalles de implementación de antd.
- **Specs**: nuevo módulo `ui-feedback`.
- **⚠️ Imagen**: el bundle de producción cambia → la imagen se republica con digest nuevo → hay que
  desplegar y reverificar el contenedor: arranque y supervivencia al presupuesto de drenaje.

## Out of Scope

- Los 11 warnings de ESLint → change separado `frontend-lint-zero`.
- `docker-compose.prod.yml` (referencia dos ficheros que no existen): fuera por decisión del usuario.
- El autoarranque del contenedor tras reinicio del host: pendiente y sin cerrar, no entra aquí.
