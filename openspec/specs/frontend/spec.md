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

### Requirement: StatsDashboard page SHALL display LLM usage summary

**Given** el usuario navega a `/stats`
**When** la página StatsDashboard se renderiza
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
**When** se renderiza StatsDashboard
**Then** la tarjeta de resumen muestra "150", "50,000", "5,000 cached", "3,000 reasoning", "$1.25", "2.0%", "1.2s"

#### Scenario: Resumen sin datos (estado vacío)
**Given** el endpoint devuelve `{total_calls: 0, total_cost: 0, ...}`
**When** se renderiza StatsDashboard
**Then** muestra "No data yet" o valores en cero
**And** no muestra gráficos vacíos

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
