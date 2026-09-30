## ADDED Requirements

### Requirement: MessageBubble SHALL fall back to UserOutlined when the user avatar fails to load

Cuando la imagen del avatar del perfil dispara un evento `error`, `MessageBubble`
SHALL dejar de renderizar la imagen y SHALL mostrar el icono `UserOutlined` en su
lugar, sin recargar la página ni perder el resto del mensaje.

**Given** un mensaje con `role = "user"` y un `userAvatarUrl` que no carga
**When** la imagen del avatar dispara un evento `error`
**Then** SHALL renderizarse el icono `UserOutlined`
**And** NO SHALL quedar visible la imagen rota

#### Scenario: URL rota degrada a UserOutlined
**Given** `message.role = "user"` y `userAvatarUrl = "https://example.com/roto.png"`
**When** se renderiza `MessageBubble` y la imagen del avatar dispara `error`
**Then** existe el icono `.anticon-user`
**And** NO existe ninguna imagen de avatar del usuario

#### Scenario: Una URL válida no degrada
**Given** `message.role = "user"` y `userAvatarUrl = "https://example.com/me.png"`
**When** se renderiza `MessageBubble` sin disparar ningún `error`
**Then** SHALL mostrarse la imagen del avatar
**And** NO SHALL mostrarse el icono `UserOutlined`

#### Scenario: Cambiar de URL recupera la imagen
**Given** un `UserAvatar` que ha degradado a `UserOutlined` por un error de carga
**When** la prop `src` cambia a una URL nueva
**Then** SHALL volver a intentarse la carga y SHALL mostrarse la imagen

### Requirement: UserAvatar SHALL avoid referrer leakage and load lazily

La imagen del avatar del perfil SHALL declarar `referrerPolicy="no-referrer"` y
`loading="lazy"` en el elemento `img`.

**Given** un `UserAvatar` con un `src` no vacío
**When** se renderiza
**Then** el elemento `img` SHALL tener `referrerPolicy="no-referrer"`
**And** el elemento `img` SHALL tener `loading="lazy"`
**And** SHALL tener un `alt` no vacío

#### Scenario: Atributos de privacidad y carga diferida
**Given** `<UserAvatar src="https://example.com/me.png" />`
**When** se renderiza
**Then** el `img` resultante tiene `referrerpolicy="no-referrer"` y `loading="lazy"`

#### Scenario: Sin src no se renderiza ninguna imagen
**Given** `<UserAvatar src={null} />`
**When** se renderiza
**Then** SHALL mostrarse `UserOutlined`
**And** NO SHALL renderizarse ningún `img`

#### Scenario: Un src en blanco se trata como ausente
**Given** `<UserAvatar src="   " />` (solo espacios)
**When** se renderiza
**Then** SHALL tratarse como si no hubiera avatar y SHALL mostrarse `UserOutlined`
**And** NO SHALL renderizarse ningún `img`

### Requirement: SettingsDialog SHALL validate the avatar URL scheme

El formulario de la pestaña Perfil SHALL aceptar en "Avatar URL" únicamente un valor
vacío, una ruta relativa del propio host (que empiece por una única `/`), o una URL
absoluta con esquema `http` o `https`. Cualquier otro esquema (por ejemplo
`javascript:`, `data:` o `file:`), una URL relativa al protocolo (que empiece por
`//`, porque apunta a un host externo) y cualquier valor que contenga caracteres de
control o espacios embebidos SHALL mostrar un error de validación y SHALL impedir el
guardado. El valor SHALL persistirse recortado de espacios al principio y al final.

**Given** el formulario de Perfil con un valor en "Avatar URL"
**When** el usuario pulsa guardar
**Then** si el esquema no es `http`/`https` y no es una ruta relativa, SHALL mostrarse
un error de validación
**And** NO SHALL llamarse a `updateProfile`

#### Scenario: Esquema no permitido muestra error y no guarda
**Given** el usuario escribe `javascript:alert(1)` en "Avatar URL"
**When** pulsa guardar
**Then** SHALL mostrarse un error de validación en ese campo
**And** NO SHALL llamarse a `updateProfile`

#### Scenario: URL https válida se guarda
**Given** el usuario escribe `https://example.com/me.png` en "Avatar URL"
**When** pulsa guardar
**Then** SHALL llamarse a `updateProfile` con esa URL
**And** NO SHALL mostrarse ningún error de validación

#### Scenario: Valor vacío sigue siendo válido
**Given** el usuario deja "Avatar URL" vacío
**When** pulsa guardar
**Then** SHALL llamarse a `updateProfile`
**And** NO SHALL mostrarse ningún error de validación

#### Scenario: Ruta relativa válida se guarda
**Given** el usuario escribe `/avatars/me.png` en "Avatar URL"
**When** pulsa guardar
**Then** SHALL llamarse a `updateProfile` con esa ruta
**And** NO SHALL mostrarse ningún error de validación

#### Scenario: El valor se persiste recortado
**Given** el usuario escribe `"  https://example.com/me.png  "` con espacios al principio y al final en "Avatar URL"
**When** pulsa guardar
**Then** SHALL llamarse a `updateProfile` con `avatar_url = "https://example.com/me.png"`, sin los espacios
**And** NO SHALL mostrarse ningún error de validación

#### Scenario: Una URL relativa al protocolo se rechaza
**Given** el usuario escribe `//evil.com/a.png` en "Avatar URL"
**When** pulsa guardar
**Then** SHALL mostrarse un error de validación en ese campo
**And** NO SHALL llamarse a `updateProfile`

#### Scenario: Caracteres de control embebidos se rechazan
**Given** el usuario escribe un valor que contiene un tabulador embebido, como "java<TAB>script:alert(1)"
**When** pulsa guardar
**Then** SHALL mostrarse un error de validación en ese campo
**And** NO SHALL llamarse a `updateProfile`
