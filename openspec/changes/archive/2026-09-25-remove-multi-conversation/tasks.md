# TDD Task Checklist — Remove Multi-Conversation

## Phase 1: Backend — DB Schema & Modelos
- [x] 1.1 Unificar migraciones en un solo archivo `00001_initial.sql` sin `conversations` ni `conversation_id`
- [x] 1.2 Actualizar `src/db/schema.rs` para que use el nuevo schema y tests no referencien `conversations`
- [x] 1.3 Eliminar `src/models/conversation.rs` y su export en `models/mod.rs`
- [x] 1.4 Eliminar `conversation_id` del modelo `Message` en `src/models/message.rs`
- [x] 1.5 Eliminar `ConversationsRepo` (`src/db/repos/conversations.rs`) y su export
- [x] 1.6 Simplificar `MessagesRepo`: eliminar verificación FK, `list_by_conversation` → `list_all`, eliminar `delete_by_conversation`

## Phase 2: Backend — Handlers & Routes
- [ ] 2.1 Eliminar `src/handlers/conversations.rs`
- [ ] 2.2 Eliminar `src/routes/conversations.rs`
- [ ] 2.3 Simplificar `src/handlers/messages.rs`: sin `conv_id` en path
- [ ] 2.4 Simplificar `src/routes/messages.rs`: paths sin `:id`
- [ ] 2.5 Simplificar `src/routes/stream.rs`: `/api/conversations/:id/messages-stream` → `/api/chat/stream`
- [ ] 2.6 Añadir `GET /api/chat/init` handler
- [ ] 2.7 Actualizar `lib.rs` (`app_with_state`) con nuevas rutas

## Phase 3: Backend — Orchestrator
- [ ] 3.1 Eliminar `conversation_id` de `process_message_stream()` y `process_message()`
- [ ] 3.2 Simplificar `list_by_token_budget` para que no filtre por `conversation_id`
- [ ] 3.3 Actualizar tests del orchestrator

## Phase 4: Backend — Tests de integración
- [ ] 4.1 Eliminar `tests/api/conversations.rs`
- [ ] 4.2 Simplificar `tests/api/messages.rs` (paths sin `conv-id`)
- [ ] 4.3 Simplificar `tests/api/chat.rs` (paths sin `conv-id`)
- [ ] 4.4 Actualizar `tests/api/common/mod.rs` (TestApp sin seed de conversations)
- [ ] 4.5 Actualizar otros tests que referencien `conversations` o `conversation_id`

## Phase 5: Frontend — Tipos y API
- [ ] 5.1 Eliminar `Conversation` de `types/index.ts` y `conversation_id` de `Message`
- [ ] 5.2 Simplificar `api/client.ts` (eliminar endpoints de conversations, simplificar messages)

## Phase 6: Frontend — Hooks
- [ ] 6.1 Eliminar `useEphemeralChat.ts`
- [ ] 6.2 Eliminar `useBrowserContext.ts` (mover lógica a useMainChat si aplica)
- [ ] 6.3 Simplificar `useMainChat.ts` (sin `conversationId`, usar `/api/chat/init`)
- [ ] 6.4 Simplificar `useSSE.ts` (sin `conversationId`)

## Phase 7: Frontend — Componentes
- [ ] 7.1 Simplificar `AppLayout.tsx` (sin sidebar, layout de una columna)
- [ ] 7.2 Actualizar `ChatView.tsx` (sin title, sin sidebar reference)
- [ ] 7.3 Actualizar `MessageBubble.tsx` (sin `conversation_id`)

## Phase 8: Frontend — Tests
- [ ] 8.1 Eliminar/actualizar tests que referencien hooks eliminados
- [ ] 8.2 Asegurar `npx tsc --noEmit` pasa
- [ ] 8.3 Asegurar `npx vitest run` pasa

## Phase 9: Verificación final
- [ ] 9.1 `cargo test` — todos los tests pasan
- [ ] 9.2 `cargo clippy -- -D warnings` — cero warnings
- [ ] 9.3 `cargo fmt --check` — formato correcto
- [ ] 9.4 `cd frontend && npx tsc --noEmit` — type check pasa
- [ ] 9.5 `cd frontend && npm run build` — build exitoso