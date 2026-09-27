## REMOVED Requirements

### Requirement: StatsDashboard page SHALL display LLM usage summary

## ADDED Requirements

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