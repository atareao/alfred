# Tasks

## 0. Línea base y red de seguridad (Phase 0 — legacy)

- [x] 0.1 Baseline: `npx eslint . -f json` (11 warnings), `npx vitest run` (16 ficheros / 116 tests), `npx tsc --noEmit` (0). Guardar salida literal.
- [x] 0.2 Pruebas de caracterización de `useTasks` (hoy sin cobertura directa): carga al montar, `refetch`, recarga por `tasks-changed`, filtros serializados. Ejecutar en GREEN.
- [x] 0.3 Pruebas de caracterización de `CalendarView` y `TaskView` (hoy sin cobertura directa). Ejecutar en GREEN.

## 1. Directiva inerte (`useSSE.ts:77`) — riesgo nulo

- [x] 1.1 Eliminar `// eslint-disable-next-line no-constant-condition`.
- [x] 1.2 Verificar que no aparece "Unused eslint-disable directive" ni ningún diagnóstico nuevo (que la directiva no estuviera suprimiendo algo real).

## 2. `react-refresh` — constantes fuera del módulo de componente (3)

- [x] 2.1 Identificar las constantes exportadas en `CalendarView.tsx:23` y `TaskView.tsx:30,39`.
- [x] 2.2 Moverlas a módulos propios y actualizar los imports de todos sus consumidores.
- [x] 2.3 GREEN: los 3 warnings desaparecidos y la suite intacta.

## 3. `exhaustive-deps` (2)

- [x] 3.1 `useMediaQuery.ts:18`: estabilizar `getMatches` con `useCallback`.
- [x] 3.2 `ChatView.tsx:41`: envolver el valor derivado condicional en `useMemo`.
- [x] 3.3 GREEN: ambos warnings desaparecidos, `useMediaQuery.test.ts` y `ChatView.test.tsx` en verde.

## 4. `set-state-in-effect` (5) — el trabajo de verdad

- [x] 4.1 RED: ejecutar la suite completa y confirmar que las pruebas de 0.2/0.3 están en verde (la red antes de tocar nada).
- [x] 4.2 `useEvents.ts:21`: sacar `setLoading(true)`/`setError(null)` del camino síncrono del efecto; `refetch` conserva su comportamiento para manejadores de evento.
- [x] 4.3 `useTasks.ts:24`: ídem.
- [x] 4.4 `useSettings.ts:33`: ídem.
- [x] 4.5 `useMainChat.ts:56`: ídem (el `setLoading(true)` del arranque).
- [x] 4.6 `StatsDashboard.tsx:55`: ídem (`loadData(selectedDays)`).
- [x] 4.7 GREEN: `npx vitest run` en verde (incluidas las de caracterización) y `npx tsc --noEmit` limpio.
- [x] 4.8 REFACTOR: `npx eslint . --max-warnings 0` con cero problemas y **cero `eslint-disable` nuevos**. Si algún punto no se puede arreglar de verdad, documentarlo y dejar el warning; no suprimir.

## 5. Ratchet del presupuesto

- [x] 5.1 Bajar el presupuesto a cero **en su única fuente de verdad**: `frontend/package.json` (`"lint:ci": "eslint . --max-warnings 0"`). El workflow no lleva presupuesto propio: invoca `npm run lint:ci`.
- [x] 5.2 Verificar el job `Frontend (Node)` en CI.

## 6. Cierre

- [x] 6.1 Verificar que ningún `*.test.ts(x)` preexistente aparece modificado en el diff.
- [x] 6.2 Marcar tareas y archivar el change.
