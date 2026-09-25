import { useState, useCallback, useEffect } from 'react';
import type { Message } from '../types';
import { api } from '../api/client';
import { useSSE } from './useSSE';

function getTimezone(): string {
  try {
    return Intl.DateTimeFormat().resolvedOptions().timeZone;
  } catch {
    return 'UTC';
  }
}

function getTimestamp(): string {
  return new Date().toISOString();
}

/**
 * Get the browser's geolocation as a Promise.
 * Falls back to null if unavailable, denied, or timed out.
 */
function getCurrentPosition(): Promise<{
  latitude: number;
  longitude: number;
} | null> {
  return new Promise((resolve) => {
    if (!navigator.geolocation) {
      resolve(null);
      return;
    }
    navigator.geolocation.getCurrentPosition(
      (pos) =>
        resolve({
          latitude: pos.coords.latitude,
          longitude: pos.coords.longitude,
        }),
      () => resolve(null),
      { enableHighAccuracy: false, timeout: 5000, maximumAge: 300000 },
    );
  });
}

export function useMainChat() {
  const [messages, setMessages] = useState<Message[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [streamingContent, setStreamingContent] = useState<string>('');
  const [streaming, setStreaming] = useState(false);
  const [activeTools, setActiveTools] = useState<string[]>([]);
  const [usedTools, setUsedTools] = useState<string[]>([]);

  const sse = useSSE();

  useEffect(() => {
    let mounted = true;
    setLoading(true);

    api.chatInit()
      .then((data) => {
        if (!mounted) return;
        setMessages(data.messages || []);
        console.log('[useMainChat] Chat initialized, messages:', data.messages?.length);
      })
      .catch((err: Error) => {
        console.error('[useMainChat] Failed to initialize chat:', err.message);
        if (mounted) setError(err.message);
      })
      .finally(() => {
        if (mounted) setLoading(false);
      });

    return () => { mounted = false; };
  }, []);

  const sendMessage = useCallback(async (content: string) => {
    console.log('[useMainChat] Sending message:', content.slice(0, 100));

    const optimistic: Message = {
      id: 'temp-' + Date.now(),
      role: 'user',
      content,
      created_at: new Date().toISOString(),
    };

    setMessages(prev => [...prev, optimistic]);
    setStreaming(true);
    setStreamingContent('');
    setError(null);
    setActiveTools([]);
    setUsedTools([]);

    let assistantContent = '';

    // Get browser context: timestamp + timezone
    const browserContext = {
      timestamp: getTimestamp(),
      timezone: getTimezone(),
      latitude: null as number | null,
      longitude: null as number | null,
      location_name: null as string | null,
    };

    // Try to get the browser's geolocation (falls back to null on denial/timeout)
    const coords = await getCurrentPosition();
    if (coords) {
      browserContext.latitude = coords.latitude;
      browserContext.longitude = coords.longitude;
      console.log('[useMainChat] Geolocation obtained:', coords.latitude, coords.longitude);
    } else {
      console.warn('[useMainChat] Geolocation unavailable or denied');
    }

    sse.connect(content, {
      onChunk: (chunk) => {
        console.log('[useMainChat] Chunk received:', chunk.slice(0, 50));
        assistantContent += chunk;
        setStreamingContent(assistantContent);
      },
      onToolCall: (name: string) => {
        console.log('[useMainChat] Tool call:', name);
        setActiveTools(prev => [...prev, name]);
        setUsedTools(prev => prev.includes(name) ? prev : [...prev, name]);
      },
      onDone: (messageId: string, userMessageId?: string) => {
        console.log('[useMainChat] Stream done. Total content length:', assistantContent.length);
        // Append tool usage footer to assistant content
        if (usedTools.length > 0) {
          assistantContent += `\n\n---\n🔧 ${usedTools.join(' · ')}`;
        }
        // Replace temp user message id with real one instead of removing it
        setMessages(prev => {
          const assistant: Message = {
            id: messageId || 'msg-' + Date.now(),
            role: 'assistant',
            content: assistantContent,
            created_at: new Date().toISOString(),
          };
          return [
            ...prev.map(m =>
              m.id === optimistic.id && userMessageId
                ? { ...m, id: userMessageId }
                : m
            ),
            assistant,
          ];
        });
        setStreaming(false);
        setStreamingContent('');
        setActiveTools([]);
        setUsedTools([]);
      },
      onError: (msg) => {
        console.error('[useMainChat] Stream error:', msg);
        setError(msg);
        setStreaming(false);
        setStreamingContent('');
        setActiveTools([]);
        setUsedTools([]);
        // Keep the user message visible - DON'T filter it out
      },
    }, browserContext);
  }, [sse]);

  return {
    messages,
    loading,
    error,
    sendMessage,
    streaming,
    streamingContent,
    activeTools,
    usedTools,
  };
}