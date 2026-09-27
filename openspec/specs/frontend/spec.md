# Frontend UI Polish — Componentes, estilos globales y tipografía

## Contracts

### Configuración de tema (theme.ts)

```typescript
export const alfredTheme: ThemeConfig = {
  token: {
    colorPrimary: '#1677ff',
    borderRadius: 6,
    colorBgContainer: '#141414',
    colorBgLayout: '#000000',
    colorText: '#ffffff',
    colorBgElevated: '#1f1f1f',
  },
  components: {
    Layout: {
      siderBg: '#001529',
      headerBg: '#141414',
      bodyBg: '#000000',
    },
    Menu: {
      darkItemBg: '#001529',
      darkItemSelectedBg: '#1677ff',
    },
  },
}
```

Sin cambios en estructura, pero se añade un token `fontSize` variable (desde settings).

### Estilos globales (global.css)

```css
/* Reset de márgenes del body que antd deja por defecto */
body {
  margin: 0;
  padding: 0;
  background: #000;
}

/* Custom scrollbar — oscura, delgada, integrada */
::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}
::-webkit-scrollbar-track {
  background: transparent;
}
::-webkit-scrollbar-thumb {
  background: rgba(255,255,255,0.15);
  border-radius: 3px;
}
::-webkit-scrollbar-thumb:hover {
  background: rgba(255,255,255,0.25);
}

/* Firefox scrollbar */
* {
  scrollbar-width: thin;
  scrollbar-color: rgba(255,255,255,0.15) transparent;
}

/* Font stack mobile-first */
:root {
  --font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto,
    'Helvetica Neue', Arial, 'Noto Sans', sans-serif, 'Apple Color Emoji',
    'Segoe UI Emoji';
  --font-size-base: 16px;
}
```

### Tipografía variable desde settings

El usuario puede configurar `font_size` (number, 12-24, default 16) en el panel de ajustes.
El valor se almacena como setting server-side (`settings.font_size`) y se aplica via CSS variable en el root del layout.

### AppLayout — sin marco blanco

El `Layout.Content` de antd NO debe tener padding/margin blanco.
Se asegura que el fondo del content es `#000` (ya configurado) y que no hay
márgenes residuales del body o de los estilos por defecto de antd.

## Escenarios

### Escenario 1: No hay marco blanco alrededor del chat
**Given** el Layout.Content con bodyBg: '#000000'  
**When** se renderiza AppLayout  
**Then** NO hay padding ni margen blanco visible  
**And** el fondo del área de chat es negro (#000)  
**And** no hay bordes ni sombras blancas alrededor del contenedor de mensajes

### Escenario 2: Scrollbar oscura integrada
**Given** la aplicación renderizada con estilos globales  
**When** el contenido del chat excede la altura visible  
**Then** la scrollbar es de 6px de ancho  
**And** el track es transparente  
**And** el thumb es rgba(255,255,255,0.15) con border-radius 3px  
**And** en Firefox usa `scrollbar-width: thin`

### Escenario 3: Tipografía mobile-first
**Given** los estilos globales cargados  
**When** se renderiza cualquier texto en la app  
**Then** la font-family usa el stack mobile-first (-apple-system, etc.)  
**And** el font-size base es 16px  
**And** en móvil (viewport < 768px) la experiencia es legible sin zoom

### Escenario 4: Usuario puede cambiar tamaño de fuente desde Ajustes
**Given** el panel de ajustes abierto  
**When** el usuario modifica `font_size` (slider de 12 a 24)  
**And** guarda los cambios  
**Then** el tamaño de fuente del chat se actualiza al valor guardado  
**And** persiste entre recargas

### Escenario 5: Valor por defecto de font_size es 16
**Given** settings sin `font_size`  
**When** se carga la app  
**Then** el font-size aplicado es 16px

## Requirements

### Requirement: CalendarView header responsivo

#### Scenario: Header se apila verticalmente en móvil
**Given** el viewport es < 768px  
**When** se abre el modal de agenda  
**Then** el selector de categoría y botón "New Event" están en columna (stack vertical)  
**And** ocupan el ancho completo disponible

#### Scenario: Header en fila en desktop
**Given** el viewport es ≥ 768px  
**When** se abre el modal de agenda  
**Then** el selector y botón están en fila horizontal (como ahora)

### Requirement: Modales con ancho dinámico

#### Scenario: EventModal se adapta en móvil
**Given** el viewport es < 768px  
**When** se abre EventModal  
**Then** el modal ocupa `'100vw'` menos padding (16px a cada lado)

#### Scenario: EventDetail se adapta en móvil
**Given** el viewport es < 768px  
**When** se abre EventDetail  
**Then** el modal ocupa `'100vw'` menos padding  
**And** las descripciones se muestran sin borde (mejor legibilidad)

### Requirement: Lista de eventos del día responsiva

#### Scenario: Eventos del día se apilan en móvil
**Given** el viewport es < 768px  
**When** se selecciona una fecha con eventos  
**Then** cada evento ocupa el ancho completo  
**And** título y hora están en vertical en vez de horizontal

### Requirement: StatsDashboard SHALL display cost and tokens by model

**Given** el endpoint `/api/stats/llm/by-model` devuelve datos
**When** se renderiza StatsDashboard
**Then** muestra un gráfico de barras (Chart.js) con coste por modelo
**And** una tabla con modelo, calls, tokens, coste

#### Scenario: Gráfico con 2 modelos
**Given** by-model devuelve `[{model: "gpt-4o", calls: 100, total_tokens: 40000, total_cost: 1.0}, {model: "claude-3", calls: 50, total_tokens: 10000, total_cost: 0.25}]`
**When** se renderiza
**Then** el gráfico tiene 2 barras
**And** la tabla tiene 2 filas ordenadas por coste descendente

### Requirement: StatsDashboard SHALL display daily time series

**Given** el endpoint `/api/stats/llm/by-day?days=30` devuelve datos
**When** se renderiza StatsDashboard
**Then** muestra un gráfico de líneas con calls/día
**And** un gráfico de líneas con coste/día
**And** un selector de rango (7d, 30d)

#### Scenario: Serie temporal de 7 días
**Given** by-day devuelve 7 puntos de datos
**When** se selecciona "7d"
**Then** el gráfico muestra 7 puntos
**And** el eje X muestra fechas en formato "DD MMM"

#### Scenario: Serie temporal de 30 días
**Given** by-day devuelve 30 puntos de datos
**When** se selecciona "30d"
**Then** el gráfico muestra 30 puntos

### Requirement: StatsDashboard SHALL display tool call frequency

**Given** el endpoint `/api/stats/llm/tools` devuelve datos
**When** se renderiza StatsDashboard
**Then** muestra un gráfico de tarta (doughnut) con las tools más llamadas
**And** una tabla con tool name y count

#### Scenario: Tools con datos
**Given** tools devuelve `[{tool: "get_weather", count: 45}, {tool: "search_web", count: 30}, {tool: "calendar", count: 15}]`
**When** se renderiza
**Then** el doughnut tiene 3 segmentos
**And** la tabla tiene 3 filas

### Requirement: StatsDashboard SHALL display database table sizes

**Given** el endpoint `/api/stats/db/sizes` devuelve datos
**When** se renderiza StatsDashboard
**Then** muestra una tabla con nombre de tabla y row count
**And** las tablas se ordenan por row count descendente

#### Scenario: DB sizes con datos
**Given** db/sizes devuelve `[{table: "messages", rows: 1500}, {table: "events", rows: 200}, ...]`
**When** se renderiza
**Then** messages aparece primero (mayor row count)

### Requirement: StatsDashboard SHALL allow CSV export

**Given** la página StatsDashboard
**When** el usuario hace clic en "Export CSV"
**Then** se descarga un archivo CSV con todos los datos de llm_requests

#### Scenario: Export exitoso
**Given** hay datos en llm_requests
**When** el usuario clica "Export CSV"
**Then** el navegador descarga `alfred-llm-requests.csv`
**And** el contenido es un CSV válido con cabeceras

### Requirement: StatsDashboard SHALL allow configuring retention days from UI

**Given** la página StatsDashboard
**When** el usuario ve la sección de configuración
**Then** muestra un control para ajustar `stats_retention_days` (número, min 7, max 365)
**And** el valor actual se carga desde `GET /api/stats/retention`
**And** al guardar se llama a `PUT /api/stats/retention` con el nuevo valor

#### Scenario: Cargar retention actual
**Given** `GET /api/stats/retention` devuelve `{"days": 30}`
**When** se renderiza la sección de configuración
**Then** el input muestra "30"

#### Scenario: Guardar retention
**Given** el usuario cambia el valor a 45
**When** hace clic en "Guardar"
**Then** se llama `PUT /api/stats/retention` con `{"days": 45}`
**And** se muestra un mensaje de confirmación

### Requirement: StatsDashboard SHALL handle loading and error states

#### Scenario: Loading state
**Given** la página StatsDashboard se está cargando
**When** los endpoints aún no han respondido
**Then** muestra un spinner o skeleton en cada sección

#### Scenario: Error state
**Given** un endpoint devuelve error 500
**When** se renderiza StatsDashboard
**Then** muestra un mensaje de error en la sección afectada
**And** el resto de secciones siguen funcionando

### Requirement: StatsDashboard dialog SHALL display LLM usage summary

StatsDashboard SHALL display LLM usage summary in a dialog modal instead of a page.

**Given** el usuario abre el diálogo Stats desde el header
**When** el modal StatsDashboard se renderiza
**Then** muestra una tarjeta de resumen global con:
- Total de llamadas al modelo
- Total de tokens (prompt + completion)
- Tokens cacheados
- Tokens de razonamiento
- Coste total en USD
- Tasa de error (%)
- Promedio de duración (si hay datos)

#### Scenario: Resumen con datos
**Given** el endpoint `/api/stats/llm/summary` devuelve `{total_calls: 150, total_tokens: 50000, total_cached_tokens: 5000, total_reasoning_tokens: 3000, total_cost: 1.25, total_errors: 3, avg_duration_ms: 1200}`
**When** se abre el diálogo Stats
**Then** la tarjeta de resumen muestra "150", "50,000", "5,000 cached", "3,000 reasoning", "$1.25", "2.0%", "1.2s"

#### Scenario: Resumen sin datos (estado vacío)
**Given** el endpoint devuelve `{total_calls: 0, total_cost: 0, ...}`
**When** se abre el diálogo Stats
**Then** muestra "No data yet" o valores en cero
**And** no muestra gráficos vacíos

### Requirement: Stats opens as Modal dialog

Stats SHALL open as a Modal dialog when clicking the bar chart icon in the header, instead of navigating to the /stats route.

**Given** el usuario está en la vista de chat principal
**When** hace clic en el icono de gráfico de barras del header
**Then** se abre un Modal titulado "📊 Stats" con el dashboard de stats
**And** la vista de chat permanece visible detrás del modal

#### Scenario: Stats dialog closes
**Given** el diálogo Stats está abierto
**When** el usuario hace clic en el botón de cerrar (X) o fuera del modal
**Then** el modal se cierra
**And** la vista de chat vuelve a estar completamente visible

#### Scenario: /stats route renders chat view
**Given** el usuario navega a `/stats`
**Then** la app renderiza la vista de chat por defecto (no el dashboard de stats)

### Requirement: SettingsDialog SHALL display Perfil tab

SettingsDialog SHALL display a "Perfil" tab with a form to edit user profile (name and avatar URL).

**Given** el SettingsDialog está abierto en la tab "Perfil"
**When** se renderiza
**Then** muestra un formulario con campos "Nombre" (Input) y "Avatar URL" (Input)
**And** los valores iniciales se cargan desde `useProfile()`

#### Scenario: Perfil se guarda correctamente
**Given** el usuario ha modificado "Nombre" a "Juan"
**When** hace clic en "Guardar"
**Then** se llama a `profile.updateProfile({ name: "Juan" })`
**And** se muestra mensaje "Perfil actualizado"
**And** el diálogo se cierra

#### Scenario: Error al guardar perfil
**Given** `updateProfile` lanza un error
**When** el usuario guarda
**Then** se muestra mensaje "Error al actualizar perfil"
**And** el diálogo permanece abierto

### Requirement: SettingsDialog SHALL display Interfaz tab

SettingsDialog SHALL display an "Interfaz" tab with font size, context window, and page size controls.

**Given** el SettingsDialog está abierto en la tab "Interfaz"
**When** se renderiza
**Then** muestra:
- "Tamaño de fuente" (InputNumber, min 12, max 24)
- "Ventana de contexto (tokens)" (InputNumber, min 1000, max 100000)
- "Tamaño de página" (InputNumber, min 10, max 100)

#### Scenario: Interfaz se guarda correctamente
**Given** el usuario cambia font_size a 18
**When** hace clic en "Guardar"
**Then** se llama a `updateSettings` con los valores actualizados
**And** se muestra mensaje "Ajustes guardados"
**And** el diálogo se cierra

### Requirement: SettingsDialog SHALL display Prompt tab

SettingsDialog SHALL display a "Prompt" tab with a textarea for the custom system prompt.

**Given** el SettingsDialog está abierto en la tab "Prompt"
**When** se renderiza
**Then** muestra un TextArea de 10 filas con el system_prompt actual
**And** un placeholder con el prompt por defecto si está vacío

#### Scenario: Prompt se guarda
**Given** el usuario escribe "Eres un asistente útil" en el TextArea
**When** hace clic en "Guardar"
**Then** `updateSettings` se llama con `{ system_prompt: "Eres un asistente útil", ... }`

### Requirement: SettingsDialog SHALL display API Keys tab

SettingsDialog SHALL display an "API Keys" tab with password fields for API keys.

**Given** el SettingsDialog está abierto en la tab "API Keys"
**When** se renderiza
**Then** muestra tres campos Input.Password:
- "OpenWeatherMap API Key"
- "Google Places API Key"
- "Brave Search API Key"

#### Scenario: API Keys se guardan
**Given** el usuario introduce una API key de OpenWeatherMap
**When** guarda
**Then** `updateSettings` se llama con la API key incluida

### Requirement: SettingsDialog SHALL unify profile and settings buttons

SettingsDialog SHALL open when clicking the settings icon in the header, replacing the separate profile and settings drawers.

**Given** el header tiene un botón con icono `SettingOutlined`
**When** el usuario hace clic en él
**Then** se abre un Modal titulado "⚙️ Settings" con tabs: Perfil, Interfaz, Prompt, API Keys
**And** el botón `UserOutlined` ya no existe en el header

#### Scenario: Dialog closes
**Given** el SettingsDialog está abierto
**When** el usuario hace clic en X o fuera del modal
**Then** el modal se cierra

#### Scenario: Modal con tabs funcionales
**Given** el SettingsDialog está abierto
**When** el usuario hace clic en la tab "API Keys"
**Then** se muestra el contenido de API Keys
**And** las otras tabs no están visibles

### Requirement: SettingsDialog SHALL handle loading and error states

#### Scenario: Restaurar valores por defecto
**Given** el usuario ha modificado valores en Interfaz
**When** hace clic en "Restaurar valores por defecto"
**Then** todos los campos vuelven a sus valores default
**And** se muestra mensaje "Valores por defecto restaurados"

### Requirement: Message TypeScript interface SHALL include location

**Given** el tipo `Message` en `frontend/src/types/index.ts`  
**When** se renderiza un mensaje  
**Then** `Message` SHALL incluir `location?: string | null`

### Requirement: ChatView SHALL show date separators between message groups

**Given** una lista de mensajes ordenados cronológicamente  
**When** se renderizan en ChatView  
**Then** entre grupos de mensajes del mismo día calendario SHALL mostrar un `DateSeparator`

#### Scenario: Misma fecha → sin separador entre mensajes consecutivos
**Given** dos mensajes con `created_at` del mismo día (ej. "2026-09-27T10:00:00Z" y "2026-09-27T11:00:00Z")  
**When** se renderizan  
**Then** NO hay separador entre ellos

#### Scenario: Fecha diferente → separador entre grupos
**Given** un mensaje con fecha "2026-09-26T23:00:00Z" y otro con "2026-09-27T01:00:00Z"  
**When** se renderizan  
**Then** aparece un `DateSeparator` con formato legible entre ambos mensajes

#### Scenario: DateSeparator muestra "Hoy" para fecha actual
**Given** la fecha actual es 2026-09-27  
**Given** un mensaje con fecha "2026-09-27T10:00:00Z"  
**When** se renderiza  
**Then** el separador muestra "Hoy"

#### Scenario: DateSeparator muestra "Ayer" para día anterior
**Given** la fecha actual es 2026-09-27  
**Given** un mensaje con fecha "2026-09-26T10:00:00Z"  
**When** se renderiza  
**Then** el separador muestra "Ayer"

#### Scenario: DateSeparator muestra fecha completa para días más antiguos
**Given** un mensaje con fecha "2026-09-25T10:00:00Z"  
**When** se renderiza  
**Then** el separador muestra "25 sept 2026"

### Requirement: MessageBubble SHALL show timestamp inline and non-intrusive

**Given** un mensaje renderizado  
**When** se visualiza  
**Then** dentro de la burbuja, junto al role label, SHALL aparecer la hora en formato `HH:mm`

#### Scenario: Timestamp formatea created_at a hora local
**Given** `message.created_at = "2026-09-27T10:30:00Z"` (UTC)  
**When** se renderiza  
**Then** se muestra `10:30` (o la hora en timezone local del settings)

#### Scenario: Timestamp con location cuando existe
**Given** `message.created_at = "2026-09-27T10:30:00Z"` y `message.location = "Silla, Valencia, España"`  
**When** se renderiza  
**Then** se muestra `10:30 · 📍 Silla` (truncado a ciudad/pueblo)

#### Scenario: Timestamp sin location
**Given** `message.created_at = "2026-09-27T10:30:00Z"` y `message.location = null`  
**When** se renderiza  
**Then** se muestra solo `10:30`

#### Scenario: Mensaje streaming no muestra timestamp
**Given** mensaje con `id = "streaming"`  
**When** se renderiza  
**Then** NO se muestra timestamp ni location
