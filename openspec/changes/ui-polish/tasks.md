# Tasks — UI Polish

## Phase 1 — SDD
- [x] Crear proposal.md con intent, scope e impacto
- [x] Crear spec.md con contratos y escenarios
- [ ] Aprobación del usuario

## Phase 2 — TDD

### Task 1: Reset global + scrollbar styling
- [x] Crear `frontend/src/global.css` con reset body, scrollbar CSS, font stack
- [x] Importar en `main.tsx`
- [x] Verificar visualmente que body margin/padding es 0
- [x] Verificar scrollbar oscura en Chrome y Firefox

### Task 2: Quitar marco blanco del Layout
- [x] En `AppLayout.tsx`, asegurar que Layout.Content no tiene padding/margin
- [x] Verificar que el fondo del chat es negro sólido
- [x] Verificar que no hay bordes blancos residuales

### Task 3: Font size en settings
- [x] En `SettingsEditor.tsx`, añadir slider `font_size` (min: 12, max: 24, step: 1)
- [x] En `ChatView.tsx`, aplicar font-size dinámico desde settings
- [x] En `AppLayout.tsx`, pasar settings a ChatView para font-size

### Task 4: Tests
- [x] Ejecutar `npm test` — 16 tests pasan
- [x] `npx tsc --noEmit` sin errores
- [x] `npm run build` exitoso