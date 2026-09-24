# Hooks Spec — f3-frontend-chat

## ADDED: useMainChat

```typescript
// frontend/src/hooks/useMainChat.ts

interface UseMainChatReturn {
  conversationId: string | null;
  messages: Message[];
  loading: boolean;
  error: string | null;
  sendMessage: (content: string) => Promise<void>;
  loadMore: () => Promise<void>;
  hasMore: boolean;
}
```

Comportamiento:
- Al montar, pide la conversación principal via `api.getMainConversation()`
- Una vez tiene el conversationId, carga los mensajes
- `sendMessage` crea un mensaje user, lo añade a la lista local optimistamente
- `loadMore` carga mensajes anteriores (paginación cursor-based)
- Es el chat principal, siempre visible, nunca se elimina

## ADDED: useEphemeralChat

```typescript
// frontend/src/hooks/useEphemeralChat.ts

interface EphemeralChat {
  id: string;
  title: string;
  messages: Message[];
  loading: boolean;
}

interface UseEphemeralChatReturn {
  chats: EphemeralChat[];
  activeChatId: string | null;
  createChat: (title: string) => Promise<string>;  // returns chat id
  sendMessage: (chatId: string, content: string) => Promise<void>;
  closeChat: (chatId: string) => void;
  selectChat: (chatId: string | null) => void;
}
```

Comportamiento:
- `createChat(title)` crea una conversación vía API y la añade a la lista local
- `sendMessage` envía mensaje a un chat efímero específico
- `closeChat` elimina la conversación vía API y la quita de la lista
- `activeChatId` = null significa que el main chat está activo
- Los chats efímeros son independientes del main chat

## ADDED: useProfile

```typescript
// frontend/src/hooks/useProfile.ts

interface UseProfileReturn {
  profile: Profile | null;
  loading: boolean;
  error: string | null;
  updateProfile: (data: UpdateProfile) => Promise<void>;
}
```

## Scenarios (BDD)

### Scenario: useMainChat loads main conversation
- **Given** the hook mounts
- **When** it calls getMainConversation()
- **Then** returns a conversationId and empty messages array

### Scenario: useMainChat sendMessage
- **Given** useMainChat has a conversationId
- **When** sendMessage('Hola') is called
- **Then** the message appears in the messages list

### Scenario: useEphemeralChat create
- **Given** useEphemeralChat is active
- **When** createChat('Tiempo en Nardó') is called
- **Then** a new ephemeral chat appears in the chats list

### Scenario: useEphemeralChat close
- **Given** an ephemeral chat exists
- **When** closeChat(id) is called
- **Then** the chat is removed from the list