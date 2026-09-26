import type { CalendarEvent, ChatInitResponse, CreateMessage, Message, PaginatedResponse, Profile, UpdateProfile } from '../types';

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
  chatInit: () =>
    request<ChatInitResponse>('/chat/init'),

  listMessages: (limit = 50, cursor?: string) =>
    request<PaginatedResponse<Message>>(
      `/messages?limit=${limit}${cursor ? `&cursor=${cursor}` : ''}`
    ),

  createMessage: (data: CreateMessage) =>
    request<Message>('/messages', {
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

  approveAction: (requestId: string, approved: boolean) =>
    request<void>(`/approval/${requestId}`, {
      method: 'POST',
      body: JSON.stringify({ approved }),
    }),

  listEvents: (start: string, end: string) =>
    request<CalendarEvent[]>(`/events?start=${encodeURIComponent(start)}&end=${encodeURIComponent(end)}`),

  createEvent: (data: Partial<CalendarEvent>) =>
    request<CalendarEvent>('/events', { method: 'POST', body: JSON.stringify(data) }),

  updateEvent: (id: string, data: Partial<CalendarEvent>) =>
    request<CalendarEvent>(`/events/${id}`, { method: 'PUT', body: JSON.stringify(data) }),

  deleteEvent: (id: string) =>
    request<void>(`/events/${id}`, { method: 'DELETE' }),
};