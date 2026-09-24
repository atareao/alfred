# API Client Spec — f3-frontend-chat

## ADDED: API Types

```typescript
// frontend/src/types/index.ts

export interface Conversation {
  id: string;
  title: string;
  created_at: string;
  updated_at: string;
}

export interface Message {
  id: string;
  conversation_id: string;
  role: 'user' | 'assistant' | 'system' | 'tool';
  content: string;
  tool_calls?: unknown;
  tool_results?: unknown;
  created_at: string;
}

export interface CreateMessage {
  role: string;
  content: string;
  tool_calls?: unknown;
  tool_results?: unknown;
}

export interface Profile {
  id: string;
  name: string;
  avatar_url?: string;
  preferences: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export interface UpdateProfile {
  name?: string;
  avatar_url?: string;
  preferences?: Record<string, unknown>;
}

export interface PaginatedResponse<T> {
  data: T[];
  next_cursor?: string;
  total?: number;
}
```

## ADDED: API Client

```typescript
// frontend/src/api/client.ts

const BASE_URL = '/api';

async function request<T>(path: string, options?: RequestInit): Promise<T> {
  const resp = await fetch(`${BASE_URL}${path}`, {
    headers: { 'Content-Type': 'application/json', ...options?.headers },
    ...options,
  });
  if (!resp.ok) {
    const error = await resp.json().catch(() => ({ error: resp.statusText }));
    throw new Error(error.error || `HTTP ${resp.status}`);
  }
  if (resp.status === 204) return undefined as T;
  return resp.json();
}

export const api = {
  // Obtiene o crea la conversación principal (la primera, o la de título "Main")
  getMainConversation: () => request<Conversation>('/conversations/main'),

  // Crea una conversación efímera para consultas puntuales
  createEphemeralConversation: (title: string) =>
    request<Conversation>('/conversations', {
      method: 'POST',
      body: JSON.stringify({ title }),
    }),

  // Messages
  listMessages: (convId: string, limit = 50, cursor?: string) =>
    request<PaginatedResponse<Message>>(
      `/conversations/${convId}/messages?limit=${limit}${cursor ? `&cursor=${cursor}` : ''}`
    ),
  createMessage: (convId: string, data: CreateMessage) =>
    request<Message>(`/conversations/${convId}/messages`, {
      method: 'POST',
      body: JSON.stringify(data),
    }),

  // Profile
  getProfile: () => request<Profile>('/profile'),
  updateProfile: (data: UpdateProfile) =>
    request<Profile>('/profile', { method: 'PUT', body: JSON.stringify(data) }),

  // Delete ephemeral conversation
  deleteConversation: (id: string) =>
    request<void>(`/conversations/${id}`, { method: 'DELETE' }),
};
```

## ADDED: Backend endpoint `/api/conversations/main`

Se necesita añadir un endpoint que devuelva o cree la conversación principal:
- Busca la primera conversación por created_at ASC
- Si no existe, la crea con título "Alfred"
- GET /api/conversations/main → Conversation

## Scenarios (BDD)

### Scenario: API client creates message
- **Given** a conversation id
- **When** `api.createMessage(convId, { role: 'user', content: 'Hola' })` is called
- **Then** returns a Message object with the given content

### Scenario: API client gets main conversation
- **Given** no conversation exists yet
- **When** `api.getMainConversation()` is called
- **Then** creates and returns a conversation with title "Alfred"

### Scenario: API client gets profile
- **Given** no profile exists
- **When** `api.getProfile()` is called
- **Then** returns a Profile with default name "Alfred User"