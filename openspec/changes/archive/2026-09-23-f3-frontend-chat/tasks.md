# Tasks — f3-frontend-chat

## TDD Task Checklist

### Backend (nuevo endpoint)
- [x] **3.0a** Añadir endpoint `GET /api/conversations/main` en backend
- [x] **3.0b** Añadir handler y test para main conversation

### Types & API
- [ ] **3.1** Crear `frontend/src/types/index.ts`
- [ ] **3.1** Crear `frontend/src/api/client.ts`

### Hooks
- [ ] **3.2** Crear `frontend/src/hooks/useMainChat.ts`
- [ ] **3.3** Crear `frontend/src/hooks/useEphemeralChat.ts`
- [ ] **3.4** Crear `frontend/src/hooks/useProfile.ts`

### Components
- [ ] **3.5** Actualizar `frontend/src/components/AppLayout.tsx` (sidebar + chat único)
- [ ] **3.6** Crear `frontend/src/components/ChatView.tsx`
- [ ] **3.6** Crear `frontend/src/components/MessageBubble.tsx`
- [ ] **3.6** Crear `frontend/src/components/MessageInput.tsx`
- [ ] **3.6** Crear `frontend/src/components/ProfileEditor.tsx`

### Tests
- [ ] **3.7** Configurar vitest + jsdom + testing-library
- [ ] **3.7** Tests: MessageBubble
- [ ] **3.7** Tests: MessageInput

### Verification
- [ ] **3.8** `npx tsc --noEmit` pasa
- [ ] **3.8** `npm run build` pasa
- [ ] **3.8** `npx vitest run` pasa
- [ ] **3.8** `cargo test` sigue pasando (legacy)