# UI Polish — Mejora de la interfaz de Alfred

## Intent
La UI de Alfred tiene tres problemas de pulido visual:
1. **Marco blanco** alrededor del chat que da sensación de caja no integrada
2. **Scrollbars por defecto del navegador** que rompen la estética oscura
3. **Tipografía sin optimizar** para móvil y sin control de tamaño

## Scope
### Frontend React (todos los cambios son en frontend/src/):
- **theme.ts**: Añadir configuración de fuente mobile-first (sistema nativa), definir estilos de scrollbar globales
- **index.html**: Añadir <style> global para scrollbar y body margin/padding reset
- **AppLayout.tsx**: Eliminar padding/margen blanco del Content, layout full-bleed
- **SettingsEditor.tsx**: Añadir slider/input para `font_size` (control numérico)
- **useSettings.ts**: Añadir soporte para `font_size` en el hook
- **ChatView.tsx**: Aplicar `fontSize` dinámico desde settings, mejorar scroll styling
- Nuevo archivo **src/global.css**: Estilos globales de scrollbar, tipografía, reset

## Impact
- **+** UI más inmersiva sin bordes blancos
- **+** Scrollbars elegantes con estilo oscuro integrado
- **+** Tipografía nativa optimizada para móvil (iOS SF, Android Roboto)
- **+** Usuario puede ajustar tamaño de fuente desde Ajustes
- **-** Mínimo: añade una dependencia de fuente (Google Fonts CDN alternativa, o usa sistema)
- **-** Settings cambia forma (nuevo campo `font_size`)