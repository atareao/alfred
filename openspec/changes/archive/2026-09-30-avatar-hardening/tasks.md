# Tasks — Robustez del avatar de usuario

## TDD checklist

### RED — Write failing tests
- [x] Scenario: URL rota degrada a `UserOutlined` (disparar `error` en la imagen)
- [x] Scenario: una URL válida no degrada
- [x] Scenario: cambiar de `src` recupera la imagen
- [x] Scenario: atributos `referrerpolicy="no-referrer"` y `loading="lazy"` en el `img`
- [x] Scenario: `<UserAvatar src={null} />` no renderiza ningún `img`
- [x] Scenario: `javascript:alert(1)` muestra error de validación y NO llama a `updateProfile`
- [x] Scenario: `https://example.com/me.png` se guarda sin error
- [x] Scenario: valor vacío sigue siendo válido
- [x] Scenario: ruta relativa `/avatars/me.png` se guarda
- [x] `cd frontend && npx vitest run` → los tests nuevos en ROJO, los legacy en VERDE

### GREEN — Minimal implementation
- [x] Crear `frontend/src/components/UserAvatar.tsx` con degradación ante `error`, sin `useEffect` de reseteo (comparar la `src` que falló con la `src` actual para no disparar `react-hooks/set-state-in-effect`)
- [x] `MessageBubble.tsx`: el rol `user` usa `<UserAvatar src={userAvatarUrl} />`
- [x] `SettingsDialog.tsx`: regla de validación en el `Form.Item` de `avatar_url` (vacío, ruta relativa o `http`/`https` permitidos; cualquier otro esquema rechazado)
- [x] `npx vitest run` → 100% verde
- [x] `npx tsc --noEmit` → sin errores

### REFACTOR
- [x] `npm run lint` → sin warnings nuevos (el repo tiene 11 preexistentes)
- [x] Re-ejecutar `npx vitest run` → sin regresiones
- [x] Verificar que el `img` del avatar tiene el mismo tamaño (24x24) y el recorte circular que antes

## Resultado

- `npx vitest run` → 19 ficheros, 120 tests, 0 fallos.
- `npx tsc --noEmit` → exit 0.
- `npx eslint` en los ficheros de la feature → 0 errores y 0 warnings.
- `npm run lint` global → 0 errores y 11 warnings, todos preexistentes en el repo y ajenos a esta feature.
- `npm run build` → correcto; el asset `valet-icon` se emite con hash.

### Incremento de endurecimiento (post-auditoría)

La auditoría de este cambio detectó 4 defectos que se cerraron en un segundo ciclo
RED→GREEN→REFACTOR dentro del mismo cambio:

- **H1 (importante):** el validador recortaba espacios pero `handleProfileSubmit`
  persistía el valor sin recortar. Un `"   "` se guardaba y `UserAvatar` emitía un
  `<img src="   ">`, provocando una petición espuria al propio SPA. Corregido con
  `avatar_url: (values.avatar_url ?? "").trim()`.
- **H2:** `//evil.com/a.png` se aceptaba como ruta relativa cuando en realidad apunta a
  un host externo (URL relativa al protocolo). Ahora se rechaza.
- **H3:** un tabulador embebido (`"java<TAB>script:alert(1)"`) burlaba el whitelist de
  esquemas, porque el tabulador no encaja en la clase de caracteres del esquema y el
  valor caía en la rama "sin esquema → válido". Ahora se rechaza cualquier valor con
  espacios o caracteres de control embebidos.
- **H4:** `UserAvatar` trataba `"   "` como un `src` válido. Ahora normaliza con
  `trim()` y una cadena en blanco se considera ausente.

Los 4 defectos se fijaron primero como tests en rojo y quedaron cubiertos:
persistencia recortada, rechazo de `//host`, rechazo de caracteres de control, y
`src` en blanco como ausente. Además se añadió un test parametrizado (`it.each`) con
`javascript:`, `data:`, `file:`, `ftp:` y `//evil.com`.

### Follow-ups conocidos y aceptados (NO incluidos)

- Caso `A(roto) → B → A` en `UserAvatar`: al volver a la misma URL que ya falló no se
  reintenta la carga hasta desmontar el componente. El escenario de la spec solo exige
  recuperarse con una URL nueva.
- `alt="Usuario"` es redundante con la metadata `user · HH:mm` de la burbuja. La spec
  exige `alt` no vacío, así que se mantiene.
