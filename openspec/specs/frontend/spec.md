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
