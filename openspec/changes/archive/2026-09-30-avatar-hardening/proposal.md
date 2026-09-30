# Robustez del avatar de usuario

## Intento
Cerrar los tres huecos detectados en la auditoría de `2026-09-30-chat-avatars`:

1. **Sin degradación ante carga fallida.** Si `avatar_url` apunta a una URL que no
   carga (404, host caído, certificado inválido), hoy el usuario ve el icono de
   imagen rota del navegador. Debe degradar a `UserOutlined`, igual que cuando no
   hay avatar configurado.
2. **Fuga de referrer e IP.** La imagen del avatar se carga desde un host externo
   sin `referrerPolicy`, lo que filtra el `Referer` y la IP del usuario al servidor
   remoto, y sin `loading="lazy"`.
3. **Esquema sin validar en Ajustes.** El campo "Avatar URL" acepta cualquier cadena
   y se usa directamente como `src`. Debe rechazarse cualquier esquema distinto de
   `http`/`https` (por ejemplo `javascript:`, `data:` o `file:`).

## Alcance
Solo frontend — no cambia backend, API, base de datos ni migraciones.

NO cambia el comportamiento ya implementado y verificado: la degradación a
`UserOutlined` con `null`/cadena vacía, ni el avatar de Valet del asistente, ni el
`ProfileProvider` compartido.

### Archivos a crear
- `frontend/src/components/UserAvatar.tsx` — componente del avatar de usuario, con
  degradación a `UserOutlined` ante error de carga.
- `frontend/src/test/UserAvatar.test.tsx` — tests del componente.

### Archivos a modificar
- `frontend/src/components/MessageBubble.tsx` — el rol `user` pasa a usar
  `<UserAvatar src={userAvatarUrl} />` en lugar del `<img>`/`<UserOutlined>` inline.
- `frontend/src/components/SettingsDialog.tsx` — validar el esquema de `avatar_url`
  en el formulario de la pestaña Perfil.
- `frontend/src/test/MessageBubble.test.tsx` — añadir el test de degradación ante
  error de carga.
- `frontend/src/test/SettingsDialog.test.tsx` — añadir el test de validación.

### Archivos a eliminar
- Ninguno

## Impacto
- Un `avatar_url` roto deja de mostrar el icono de imagen rota del navegador: cae a
  `UserOutlined` en cuanto la imagen dispara `error`.
- El navegador NO envía `Referer` al host externo del avatar y la imagen se carga
  de forma diferida.
- Guardar un `avatar_url` con esquema no permitido muestra un error de validación
  en el formulario y NO persiste el cambio.
- Sigue aceptándose `avatar_url` vacío (comportamiento por defecto) y una ruta
  relativa del propio host, además de URLs absolutas `http`/`https`.
- Sin dependencias nuevas.

## Riesgos
- Los tests ya existentes de `MessageBubble` comprueban `img[src="..."]` para el
  aviso con avatar y `.anticon-user` para el usuario sin avatar; ambos deben seguir
  pasando sin tocarse.
- La validación NO debe romper el guardado con `avatar_url` vacío, que es el caso
  que usan los tests actuales de `SettingsDialog`.
- Si se generaliza el `referrerPolicy` al logo de Valet sería un cambio inútil: el
  logo es un asset local del propio bundle. Solo aplica al avatar del perfil.
