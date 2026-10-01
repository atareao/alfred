# Proposal: Deuda de lint a cero (frontend-lint-zero)

## Why

El job `Frontend (Node)` corre ESLint con `--max-warnings 11`. Ese número no es un umbral de
calidad: es la foto del día en que se configuró el CI, congelada para no bloquear el trabajo en
curso. El efecto es el contrario del buscado: mientras el total no pase de 11, **cualquier warning
nuevo entra gratis**. El presupuesto se ha convertido en el techo.

Los 11 warnings no son once problemas iguales. Son cuatro familias con riesgos muy distintos:

| Familia | Nº | Regla | Dónde | Riesgo al arreglar |
|---|---|---|---|---|
| Carga de datos en efecto | **5** | `react-hooks/set-state-in-effect` | `useEvents:21`, `useTasks:24`, `useSettings:33`, `useMainChat:56`, `StatsDashboard:55` | **Alto** — toca la carga de datos de la app |
| Dependencias de efecto | 2 | `react-hooks/exhaustive-deps` | `useMediaQuery:18`, `ChatView:41` | Medio — cambia cuándo se re-suscribe/recalcula |
| Fast refresh roto | 3 | `react-refresh/only-export-components` | `CalendarView:23`, `TaskView:30`, `TaskView:39` | Bajo — movimiento mecánico de constantes |
| Directiva inerte | 1 | `Unused eslint-disable directive` | `useSSE:77` | Nulo |

### El caso que importa: los cinco `set-state-in-effect`

Los cinco tienen el mismo patrón, y lo tienen desde antes de que existiera la regla:

```ts
const refetch = useCallback(() => {
  setLoading(true);        // ← síncrono, antes de cualquier await
  setError(null);
  api.listEvents(start, end).then(...).catch(...).finally(...);
}, [start, end]);

useEffect(() => {
  refetch();               // ← el efecto llama a algo que hace setState síncrono
}, [refetch]);
```

La regla (la nueva del plugin de React, orientada al React Compiler) marca exactamente eso:
`setState` síncrono dentro del cuerpo del efecto provoca render en cascada.

**No es un falso positivo y no se va a silenciar.** Es un patrón con consecuencias reales:
`StatsDashboard` re-ejecuta `loadData` en cada cambio de `selectedDays` fijando estado síncrono, y
`useEvents`/`useTasks` hacen lo mismo al montar. La corrección honesta es sacar las marcas de
"petición en curso" del camino síncrono del efecto (el estado inicial ya cubre la primera carga) y
dejar que el estado se actualice solo al resolverse la petición. `refetch` conserva su
comportamiento síncrono, que **fuera** de un efecto (manejadores de evento) es legítimo.

Los otros seis se arreglan de verdad, sin supresiones: dos dependencias mal declaradas, tres
constantes que rompen el fast refresh, y una directiva muerta.

## What Changes

### 1. `set-state-in-effect` — 5 puntos (arreglo real, sin supresiones)

Sacar el `setState` síncrono del camino del efecto en `useEvents.ts`, `useTasks.ts`,
`useSettings.ts`, `useMainChat.ts` y `StatsDashboard.tsx`, preservando la carga al montar, la
recarga al cambiar dependencias, el `refetch` expuesto y las recargas por evento
(`events-changed`, `tasks-changed`).

### 2. `exhaustive-deps` — 2 puntos

Estabilizar `getMatches` con `useCallback` en `useMediaQuery.ts` y envolver `allMessages` en
`useMemo` en `ChatView.tsx`.

### 3. `react-refresh` — 3 puntos

Mover las constantes exportadas de `CalendarView.tsx` y `TaskView.tsx` a su propio módulo, dejando
esos ficheros exportando solo componentes.

### 4. Directiva inerte — 1 punto

Eliminar la directiva `eslint-disable-next-line no-constant-condition` de `useSSE.ts`.

### 5. Ratchet del presupuesto

`--max-warnings 11` → `--max-warnings 0` en `.github/workflows/ci.yml`, para que la deuda no pueda
volver a crecer.

## Impact

- **Ficheros**: `useEvents.ts`, `useTasks.ts`, `useSettings.ts`, `useMainChat.ts`,
  `useMediaQuery.ts`, `useSSE.ts`, `ChatView.tsx`, `CalendarView.tsx`, `TaskView.tsx`,
  `StatsDashboard.tsx`, `.github/workflows/ci.yml` y dos módulos de constantes nuevos.
- **Comportamiento**: ninguno observable. Es el invariante central del change y se verifica
  exigiendo que **ninguna aserción existente cambie**.
- **Cobertura**: `useTasks`, `CalendarView` y `TaskView` no tienen pruebas hoy. Se añaden pruebas de
  caracterización **antes** de tocarlos (Phase 0): sin red, refactorizar carga de datos es apostar.
- **Estilo de lint**: cero supresiones nuevas. Si un punto no se puede arreglar de verdad, se
  documenta y se deja el warning; no se borra el síntoma.
- **Specs**: nuevo módulo `frontend-lint`.
- **Imagen**: el bundle del frontend cambia → la imagen se republica → requiere redespliegue.

## Out of Scope

- El refactor de la API contextual de antd → change separado `antd-message-context`.
- `docker-compose.prod.yml` (referencia dos ficheros que no existen): sigue fuera, por decisión del
  usuario.
- El autoarranque del contenedor tras reinicio del host: pendiente y sin cerrar, no entra aquí.
