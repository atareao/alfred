# Eliminar los tests de depuración sin aserciones

## Intento
Eliminar los cuatro ficheros `debug*.test.tsx` de `frontend/src/components/`. No contienen
ninguna aserción: solo montan `AppLayout` y escriben en consola. No pueden fallar salvo que
el componente reviente al montarse, así que no aportan señal de calidad, ensucian la salida
de la suite con ruido de consola y mantienen vivo código de depuración que nunca se limpió.

Evidencia (verificada):

| Fichero | Líneas | `expect(` | `console.log` |
|---|---|---|---|
| `debug.test.tsx` | 42 | 0 | 6 |
| `debug2.test.tsx` | 37 | 0 | 4 |
| `debug3.test.tsx` | 40 | 0 | 4 |
| `debug4.test.tsx` | 54 | 0 | 6 |

Provienen del commit `5dee8c2 release: v0.7.0`: son deuda previa, no forman parte de ningún
cambio activo. En el cambio anterior se les envolvió en `ProfileProvider` únicamente para
restaurar su verde tras introducir el contexto de perfil; ese trabajo deja de ser necesario
si los ficheros desaparecen.

## Alcance
Solo frontend — no cambia backend, API, base de datos ni migraciones.

### Archivos a eliminar
- `frontend/src/components/debug.test.tsx`
- `frontend/src/components/debug2.test.tsx`
- `frontend/src/components/debug3.test.tsx`
- `frontend/src/components/debug4.test.tsx`

### Archivos a modificar
- Ninguno

## Impacto
- La suite pasa de 19 ficheros / 120 tests a 15 ficheros / 115 tests.
- Cobertura real: sin cambios, porque ninguno de los 5 tests eliminados tenía aserciones.
- Desaparece el ruido de `console.log` en la salida de `vitest`.
- `AppLayout.test.tsx` se conserva: es el que cubre `AppLayout` con aserciones reales (6 tests).
- Sin dependencias nuevas ni cambios de comportamiento en la aplicación.

## Fuera de alcance (evaluado y descartado por ahora)
- **Separar `isSaving` de `loading` en `useProfile`:** hoy ningún consumidor lee `loading` del
  contexto, así que sería trabajo especulativo. Se abordará cuando exista un consumidor real.
- **Reintento de carga en `UserAvatar` al volver a una URL que ya falló (`A → B → A`):** la
  auditoría lo declaró aceptable; solo afecta a volver hacia atrás a un avatar que previamente
  falló y se resuelve al desmontar el componente. Sin impacto observable hoy.

## Verificación
- `npx vitest run` → 15 ficheros, 115 tests, 0 fallos (antes: 19 / 120).
- `npx tsc --noEmit` → sin errores.
- `npm run lint` → 11 warnings preexistentes, 0 errores (sin cambios).
- `grep -rn "debug.test\|debug2.test\|debug3.test\|debug4.test" frontend/` → sin referencias.
