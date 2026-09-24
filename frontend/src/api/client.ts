import type { Conversation, CreateMessage, Message, PaginatedResponse, Profile, UpdateProfile } from '../types';

export const BASE_URL = '/api';

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
  getMainConversation: () =>
    request<Conversation>('/conversations/main'),

  createEphemeralConversation: (title: string) =>
    request<Conversation>('/conversations', {
      method: 'POST',
      body: JSON.stringify({ title }),
    }),

  listMessages: (convId: string, limit = 50, cursor?: string) =>
    request<PaginatedResponse<Message>>(
      `/conversations/${convId}/messages?limit=${limit}${cursor ? `&cursor=${cursor}` : ''}`
    ),

  createMessage: (convId: string, data: CreateMessage) =>
    request<Message>(`/conversations/${convId}/messages`, {
      method: 'POST',
      body: JSON.stringify(data),
    }),

  getProfile: () => request<Profile>('/profile'),
  updateProfile: (data: UpdateProfile) =>
    request<Profile>('/profile', { method: 'PUT', body: JSON.stringify(data) }),

  getSettings: () => request<Record<string, string>>('/settings'),

  updateSettings: (data: Record<string, string>) =>
    request<Record<string, string>>('/settings', {
      method: 'PUT',
      body: JSON.stringify(data),
    }),

  deleteConversation: (id: string) =>
    request<void>(`/conversations/${id}`, { method: 'DELETE' }),

  approveAction: (requestId: string, approved: boolean) =>
    request<void>(`/approval/${requestId}`, {
      method: 'POST',
      body: JSON.stringify({ approved }),
    }),
};