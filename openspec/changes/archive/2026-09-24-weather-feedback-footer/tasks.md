# Weather + Feedback + Footer — Task Checklist

## TDD Tasks

### Grupo A: Backend
- [x] **1.1** Fix parseo de fechas en `weather.rs` (aceptar "YYYY-MM-DD" sin hora)
- [x] **1.2** Trackear tools usadas + footer en `agent.rs` (streaming path)
- [x] **1.3** `cargo test`, `cargo clippy`, `cargo fmt`

### Grupo B: Frontend
- [ ] **2.1** `useMainChat.ts`: añadir `onToolCall` para trackear tools durante streaming
- [ ] **2.2** `ChatView.tsx`: mostrar "🔧 Ejecutando tool..."
- [ ] **2.3** `MessageBubble.tsx`: mostrar footer de tools
- [ ] **2.4** `npx tsc --noEmit`, `npm run build`

### Grupo C: Verificación final
- [ ] **3.1** `cargo test` — todos pasan
- [ ] **3.2** `cargo clippy -- -D warnings` — cero warnings
- [ ] **3.3** Frontend build sin errores