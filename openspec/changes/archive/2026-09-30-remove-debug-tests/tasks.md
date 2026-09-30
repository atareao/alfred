# Tasks — Eliminar tests de depuración sin aserciones

## Checklist

### Verificación previa
- [x] Confirmar que los 4 ficheros tienen 0 aserciones: `grep -c 'expect(' frontend/src/components/debug*.test.tsx`
- [x] Anotar el conteo inicial de la suite: `cd frontend && npx vitest run` → 19 ficheros / 120 tests
- [x] Confirmar que `AppLayout.test.tsx` cubre `AppLayout` con aserciones reales, para no perder cobertura al eliminar los debug

### Eliminación
- [x] `git rm frontend/src/components/debug.test.tsx`
- [x] `git rm frontend/src/components/debug2.test.tsx`
- [x] `git rm frontend/src/components/debug3.test.tsx`
- [x] `git rm frontend/src/components/debug4.test.tsx`

### Verificación posterior
- [x] `cd frontend && npx vitest run` → 15 ficheros, 115 tests, 0 fallos
- [x] `cd frontend && npx tsc --noEmit` → sin errores
- [x] `cd frontend && npm run lint` → 11 warnings preexistentes, 0 errores
- [x] Confirmar que la salida de `vitest` ya no muestra los `console.log` de depuración
- [x] `grep -rn "debug.test\|debug2.test\|debug3.test\|debug4.test" frontend/` → sin referencias

## Resultado

- ANTES: `npx vitest run` → 19 ficheros, 120 tests, 0 fallos.
- DESPUÉS: `npx vitest run` → 15 ficheros, 115 tests, 0 fallos.
- `npx tsc --noEmit` → exit 0.
- `npm run lint` → 0 errores y 11 warnings preexistentes (recuento sin cambios).
- Eliminados con `git rm`: `debug.test.tsx`, `debug2.test.tsx`, `debug3.test.tsx`, `debug4.test.tsx`.
- Los 4 ficheros sumaban 173 líneas, 0 aserciones y 20 `console.log`. Ninguno exportaba
  helpers ni mocks compartidos: solo montaban `AppLayout` con mocks duplicados inline.
- `frontend/src/components/AppLayout.test.tsx` se conserva (8 aserciones reales), por lo que
  la cobertura de `AppLayout` no se pierde: los tests eliminados no verificaban nada.
- La salida de `vitest` ya no muestra el ruido de consola de depuración.

### Hallazgo colateral (NO tratado en este cambio)

- `frontend/.vitest/json/output.json` está **trackeado en git** (19 KB) y contiene rutas de
  otro proyecto (`/data/rust/alfred`), además de referencias a los ficheros de test ahora
  eliminados. Es un artefacto generado que no debería estar bajo control de versiones.
  Última modificación: commit `73d9696`. Fuera del alcance de este cambio; candidato a
  eliminarse del repo y añadirse a `.gitignore` en un cambio aparte.

### Follow-ups evaluados y descartados (decidido, no olvidado)

- Separar `isSaving` de `loading` en `useProfile`: ningún consumidor lee `loading` del
  contexto hoy. Trabajo especulativo; se hará cuando exista un consumidor real.
- Reintento de carga `A → B → A` en `UserAvatar`: la auditoría lo declaró aceptable y solo
  afecta a volver hacia atrás a una URL que ya falló.
