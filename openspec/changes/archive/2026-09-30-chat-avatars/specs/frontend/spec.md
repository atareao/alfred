## ADDED Requirements

### Requirement: MessageBubble SHALL render the Valet logo as assistant avatar

Los mensajes con rol `assistant` SHALL mostrar el logotipo de Valet (SVG vectorial)
como avatar a la izquierda de la burbuja, en lugar del icono genérico
`RobotOutlined` de Ant Design.

**Given** un mensaje con `role = "assistant"`
**When** `MessageBubble` se renderiza
**Then** SHALL aparecer el logo de Valet como avatar a la izquierda de la burbuja
**And** NO SHALL renderizarse el icono `RobotOutlined`

#### Scenario: Mensaje del asistente muestra el logo de Valet
**Given** `message.role = "assistant"` y `message.content = "Hola, soy Valet"`
**When** se renderiza `MessageBubble`
**Then** existe una imagen (`<img>`) cuyo `src` apunta al asset del logo de Valet
**And** esa imagen tiene un `alt` no vacío
**And** NO existe un elemento con la clase del icono robot de Ant Design

#### Scenario: Mensaje de streaming usa el mismo avatar
**Given** `message.id = "streaming"` y `message.role = "assistant"`
**When** se renderiza `MessageBubble`
**Then** el avatar SHALL ser también el logo de Valet
**And** NO SHALL renderizarse el icono `RobotOutlined`

#### Scenario: El rol system y el rol tool conservan su icono
**Given** mensajes con `role = "system"` y `role = "tool"`
**When** se renderizan
**Then** SHALL mantener sus iconos actuales (`InfoCircleOutlined`, `CodeOutlined`)
**And** NO SHALL mostrarse el logo de Valet

### Requirement: MessageBubble SHALL render the user profile avatar in user messages

Los mensajes con rol `user` SHALL mostrar como avatar la imagen del perfil recibida
en la prop `userAvatarUrl`, a la derecha de la burbuja. Si `userAvatarUrl` es
`null`, `undefined` o cadena vacía, SHALL mantener el icono `UserOutlined` actual.

**Given** un mensaje con `role = "user"`
**When** `MessageBubble` se renderiza
**Then** SHALL usar `userAvatarUrl` como avatar si está disponible
**And** SHALL degradar a `UserOutlined` si no lo está

#### Scenario: Usuario con avatar configurado
**Given** `message.role = "user"` y `userAvatarUrl = "https://example.com/me.png"`
**When** se renderiza `MessageBubble`
**Then** existe una imagen con `src = "https://example.com/me.png"` y `alt` no vacío
**And** NO existe el icono `anticon-user`

#### Scenario: Usuario sin avatar configurado (degradación)
**Given** `message.role = "user"` y `userAvatarUrl = null`
**When** se renderiza `MessageBubble`
**Then** SHALL renderizarse el icono `UserOutlined`
**And** NO SHALL renderizarse ninguna imagen de avatar de usuario

#### Scenario: El avatar del usuario NO se usa en otros roles
**Given** `userAvatarUrl = "https://example.com/me.png"`
**And** mensajes con `role = "assistant"`, `role = "system"` y `role = "tool"`
**When** se renderizan
**Then** NO SHALL usarse `userAvatarUrl` en ninguno de ellos

### Requirement: ChatView SHALL forward the user avatar URL to message bubbles

`ChatView` SHALL aceptar una prop opcional `userAvatarUrl?: string | null` y
reenviarla a cada `MessageBubble` que renderice, incluido el mensaje sintético de
streaming.

**Given** un `ChatView` con `userAvatarUrl` disponible
**When** se renderiza
**Then** cada `MessageBubble` SHALL recibir esa misma URL
**And** los mensajes con `role = "user"` SHALL mostrar ese avatar

#### Scenario: Propagación de la URL a las burbujas
**Given** `messages` con un mensaje de rol `user` y `userAvatarUrl = "https://example.com/me.png"`
**When** se renderiza `ChatView`
**Then** la burbuja del mensaje de usuario SHALL mostrar una imagen con ese `src`

#### Scenario: Prop ausente no rompe el render
**Given** un `ChatView` renderizado sin la prop `userAvatarUrl`
**When** se renderiza
**Then** los mensajes con `role = "user"` SHALL mostrar `UserOutlined`
**And** NO SHALL lanzarse ningún error

### Requirement: ProfileProvider SHALL provide a single shared profile to the whole app

El perfil del usuario SHALL obtenerse una sola vez y compartirse mediante un
contexto React (`ProfileProvider` + `useProfileContext()`), de modo que
`SettingsDialog` y el chat consuman la misma instancia y no haya copias
desincronizadas.

**Given** la app envuelta en `<ProfileProvider>`
**When** un consumidor llama a `useProfileContext()`
**Then** recibe `{ profile, loading, error, updateProfile }`
**And** solo SHALL realizarse una petición `GET /api/profile` en toda la app

#### Scenario: Uso fuera del provider falla explícitamente
**Given** un componente que llama a `useProfileContext()` sin un `<ProfileProvider>` ancestro
**When** se renderiza
**Then** SHALL lanzarse un error indicando que falta el provider

#### Scenario: Editar el avatar en Ajustes actualiza el chat sin recargar
**Given** el usuario cambia "Avatar URL" en la pestaña Perfil y pulsa guardar
**When** `updateProfile` resuelve con el perfil actualizado
**Then** el contexto SHALL exponer el nuevo `avatar_url`
**And** los mensajes con `role = "user"` del chat SHALL mostrar el nuevo avatar sin recargar la página

#### Scenario: Proveedor único
**Given** `SettingsDialog` y `AppLayout` renderizados bajo el mismo `<ProfileProvider>`
**When** ambos consumen el perfil
**Then** SHALL compartir la misma instancia de perfil
**And** NO SHALL duplicarse la petición a `GET /api/profile`
