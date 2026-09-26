# Change Proposal: CalendarView responsive

## Why

El modal de agenda no se adapta bien a pantallas pequeñas:
- Header con selector de categoría (160px) + botón "New Event" no caben en móvil
- Ant Design Calendar tiene celdas minúsculas en viewports pequeños
- El modal de detalle (`width={480}`) y el de crear evento (`width={520}`) se desbordan
- La lista de eventos del día con `Space` no wrappean correctamente

## What Changes

### CalendarView
- Header responsivo: en móvil (< 768px) el selector y botón pasan a columna (stack vertical)
- El calendario usa `fullscreen={false}` en móvil para mostrar solo el mes
- La lista de eventos del día usa stack vertical en móvil en vez de `Space` horizontal

### EventModal
- `width` dinámico: 520 en desktop, `'100vw'` con padding en móvil
- Formularios se adaptan al ancho disponible

### EventDetail
- `width` dinámico: 480 en desktop, `'100vw'` con padding en móvil
- Usar `column={1}` siempre pero sin `bordered` en móvil para mejor legibilidad

## Scope
- Solo frontend: CalendarView, EventModal, EventDetail
- Helper: `useMediaQuery` hook o utilidad inline (window.matchMedia)

## Impact
- Sin breaking changes en desktop
- Mejora progresiva — mobile-first