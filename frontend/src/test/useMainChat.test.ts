import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useMainChat } from '../hooks/useMainChat';

// ---------------------------------------------------------------------------
// Hoisted shared storage — allows mock modules to capture SSE callbacks that
// the test can then invoke to simulate stream completion / error.
// ---------------------------------------------------------------------------
const { sseCallbacks } = vi.hoisted(() => ({ sseCallbacks: {} as Record<string, unknown> }));
const { capturedBrowserContext } = vi.hoisted(() => ({ capturedBrowserContext: { current: undefined as unknown } }));

vi.mock('../api/client', () => ({
  api: {
    getMainConversation: vi.fn().mockResolvedValue({ id: 'conv-1' }),
    listMessages: vi.fn().mockResolvedValue({ data: [], next_cursor: null }),
  },
  BASE_URL: 'http://localhost:3000',
}));

vi.mock('../hooks/useSSE', () => ({
  useSSE: vi.fn(() => ({
    connect: vi.fn((_convId: string, _content: string, options: { onChunk?: (c: string) => void; onDone?: (...args: string[]) => void; onError?: (m: string) => void }, browserContext?: unknown) => {
      sseCallbacks.onChunk = options.onChunk;
      sseCallbacks.onDone = options.onDone;
      sseCallbacks.onError = options.onError;
      capturedBrowserContext.current = browserContext;
    }),
    disconnect: vi.fn(),
    connected: false,
  })),
}));

vi.mock('../hooks/useBrowserContext', () => ({
  useBrowserContext: vi.fn(() => ({
    context: {
      timestamp: '2026-09-24T08:00:00.000Z',
      timezone: 'Europe/Madrid',
      latitude: 39.36,
      longitude: -0.41,
      location_name: 'Silla, Valencia, España',
    },
    error: null,
    permission: 'granted' as const,
    refresh: vi.fn(),
  })),
}));

describe('useMainChat', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Clear captured callbacks between tests
    sseCallbacks.onChunk = undefined;
    sseCallbacks.onDone = undefined;
    sseCallbacks.onError = undefined;
  });

  // -----------------------------------------------------------------------
  // Bug #2 — User message deleted on stream complete
  //
  // Current (broken) onDone in useMainChat:
  //   return [...prev.filter(m => m.id !== optimistic.id), assistant];
  //
  // Expected (fixed) onDone should PRESERVE the user message and update its
  // ID with the server-assigned ID.
  // -----------------------------------------------------------------------
  it('preserves user message when stream completes', async () => {
    const { result } = renderHook(() => useMainChat());

    // Wait for the initial conversation load to finish
    await vi.waitFor(() => {
      expect(result.current.loading).toBe(false);
    }, { timeout: 3000 });

    // Verify we have a conversation ID
    expect(result.current.conversationId).toBe('conv-1');

    // Send a message — this adds the optimistic user message and calls SSE.connect
    await act(async () => {
      result.current.sendMessage('Hola');
    });

    // After sendMessage, the optimistic user message should be in the list
    expect(result.current.messages).toHaveLength(1);
    expect(result.current.messages[0].role).toBe('user');
    expect(result.current.messages[0].content).toBe('Hola');
    expect(result.current.streaming).toBe(true);

    // Capture the optimistic ID so we can verify it survives the onDone call
    const optimisticId = result.current.messages[0].id;
    expect(optimisticId).toMatch(/^temp-/);

    // Simulate the SSE onDone callback
    await act(async () => {
      const doneFn = sseCallbacks.onDone as (() => void);
      doneFn();
    });

    // ------------------------------------------------------------------
    // RED phase assertion — this SHOULD FAIL with current broken code.
    //
    // The current code does:
    //   return [...prev.filter(m => m.id !== optimistic.id), assistant];
    //
    // which removes the user message. After the fix, the user message
    // should remain (its ID gets replaced with the server-assigned ID).
    // ------------------------------------------------------------------
    const userMessages = result.current.messages.filter(m => m.role === 'user');
    expect(userMessages).toHaveLength(1);
    expect(userMessages[0].content).toBe('Hola');
  });

  // -----------------------------------------------------------------------
  // Bug #2 — User message deleted on stream error
  //
  // Current (broken) onError in useMainChat:
  //   setMessages(prev => prev.filter(m => m.id !== optimistic.id));
  //
  // Expected: the user message should remain visible (as a failed message).
  // -----------------------------------------------------------------------
  it('preserves user message when stream errors', async () => {
    const { result } = renderHook(() => useMainChat());

    // Wait for initial load
    await vi.waitFor(() => {
      expect(result.current.loading).toBe(false);
    }, { timeout: 3000 });

    // Send a message
    await act(async () => {
      result.current.sendMessage('Hola');
    });

    expect(result.current.messages).toHaveLength(1);
    expect(result.current.messages[0].role).toBe('user');

    // Simulate SSE onError
    await act(async () => {
      const errorFn = sseCallbacks.onError as (msg: string) => void;
      errorFn('Stream error');
    });

    // ------------------------------------------------------------------
    // RED phase assertion — this SHOULD FAIL with current broken code.
    //
    // The current code filters out the user message on error.
    // After the fix, the user message should remain visible.
    // ------------------------------------------------------------------
    const userMessages = result.current.messages.filter(m => m.role === 'user');
    expect(userMessages).toHaveLength(1);
    expect(userMessages[0].content).toBe('Hola');
  });

  // -----------------------------------------------------------------------
  // context-personality: Browser context passed to SSE connect
  //
  // After the change, useMainChat should call useBrowserContext() internally
  // and pass the browserContext object as the 4th argument to sse.connect().
  // This test will FAIL in RED phase because useMainChat does not yet use
  // useBrowserContext.
  // -----------------------------------------------------------------------
  it('sends message with browser context', async () => {
    const { result } = renderHook(() => useMainChat());

    // Wait for the initial conversation load to finish
    await vi.waitFor(() => {
      expect(result.current.loading).toBe(false);
    }, { timeout: 3000 });

    // Send a message
    await act(async () => {
      result.current.sendMessage('Hola');
    });

    // ------------------------------------------------------------------
    // RED phase assertion — this SHOULD FAIL because the current
    // useMainChat does not import useBrowserContext, so capturedBrowserContext
    // will be undefined.
    //
    // After the GREEN phase, useMainChat will:
    //   1. Import and call useBrowserContext()
    //   2. Pass browserContext as the 4th arg to sse.connect()
    //   3. capturedBrowserContext will be a valid BrowserContext object
    // ------------------------------------------------------------------
    expect(capturedBrowserContext.current).toBeTruthy();
    expect(capturedBrowserContext.current).toHaveProperty('timestamp');
    expect(capturedBrowserContext.current).toHaveProperty('timezone');
    expect(capturedBrowserContext.current).toHaveProperty('latitude');
    expect(capturedBrowserContext.current).toHaveProperty('longitude');
    expect(capturedBrowserContext.current).toHaveProperty('location_name');
  });
});