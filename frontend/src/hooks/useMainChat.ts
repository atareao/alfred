import { useState, useCallback, useEffect } from 'react';
import type { Message } from '../types';
import { api } from '../api/client';
import { useSSE } from './useSSE';
import { useBrowserContext } from './useBrowserContext';

export function useMainChat() {
  const [conversationId, setConversationId] = useState<string | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [hasMore, setHasMore] = useState(false);
  const [cursor, setCursor] = useState<string | undefined>(undefined);
  const [streamingContent, setStreamingContent] = useState<string>('');
  const [streaming, setStreaming] = useState(false);
  const [activeTools, setActiveTools] = useState<string[]>([]);
  const [usedTools, setUsedTools] = useState<string[]>([]);

  const sse = useSSE();
  const { context: browserContext } = useBrowserContext();

  useEffect(() => {
    let mounted = true;
    setLoading(true);
    api.getMainConversation()
      .then(conv => {
        if (!mounted) return;
        setConversationId(conv.id);
        console.log('[useMainChat] Conversation loaded:', conv.id);
        return api.listMessages(conv.id, 50).then(resp => {
          if (!mounted) return;
          setMessages(resp.data);
          setHasMore(resp.next_cursor != null);
          setCursor(resp.next_cursor);
        });
      })
      .catch(err => {
        console.error('[useMainChat] Failed to load conversation:', err.message);
        if (mounted) setError(err.message);
      })
      .finally(() => {
        if (mounted) setLoading(false);
      });
    return () => { mounted = false; };
  }, []);

  const sendMessage = useCallback(async (content: string) => {
    if (!conversationId) return;
    console.log('[useMainChat] Sending message:', { conversationId, content: content.slice(0, 100) });

    const optimistic: Message = {
      id: 'temp-' + Date.now(),
      conversation_id: conversationId,
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

    sse.connect(conversationId, content, {
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
            conversation_id: conversationId,
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
  }, [conversationId, sse, browserContext]);

  const loadMore = useCallback(async () => {
    if (!conversationId || !cursor) return;
    try {
      const resp = await api.listMessages(conversationId, 50, cursor);
      setMessages(prev => [...resp.data, ...prev]);
      setHasMore(resp.next_cursor != null);
      setCursor(resp.next_cursor);
    } catch (err: any) {
      setError(err.message);
    }
  }, [conversationId, cursor]);

  return {
    conversationId,
    messages,
    loading,
    error,
    sendMessage,
    loadMore,
    hasMore,
    streaming,
    streamingContent,
    activeTools,
    usedTools,
  };
}