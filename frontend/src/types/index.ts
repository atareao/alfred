export interface Message {
  id: string;
  role: 'user' | 'assistant' | 'system' | 'tool';
  content: string;
  tool_calls?: unknown;
  tool_results?: unknown;
  tokens_count?: number;
  collapsed_content?: string | null;
  collapsed_tokens_count?: number;
  is_indexed?: boolean;
  summary_ref?: string | null;
  created_at: string;
}

export interface CreateMessage {
  role: string;
  content: string;
  tool_calls?: unknown;
  tool_results?: unknown;
}

export interface Settings {
  max_window_tokens: string;
  system_prompt: string;
  [key: string]: string;
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

export interface BrowserContext {
  timestamp: string;
  timezone: string;
  latitude: number | null;
  longitude: number | null;
  location_name: string | null;
}

export interface MessageQuery {
  content: string;
  browser_context?: BrowserContext;
  override?: string;
}

export type SSEEventType = 'chunk' | 'tool_call' | 'tool_result' | 'done' | 'error' | 'approval_required' | 'approval_result';

export interface SSEStreamEvent {
  type: SSEEventType;
  content?: string;
  name?: string;
  args?: unknown;
  success?: boolean;
  message_id?: string;
  user_message_id?: string;
  message?: string;
  request_id?: string;
  tool_name?: string;
  reason?: string;
  approved?: boolean;
}

export interface ChatInitResponse {
  messages: Message[];
  settings: Record<string, string>;
}