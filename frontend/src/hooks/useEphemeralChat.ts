import { useState, useCallback } from 'react';
import type { Message } from '../types';
import { api } from '../api/client';

interface EphemeralChat {
  id: string;
  title: string;
  messages: Message[];
  loading: boolean;
}

export function useEphemeralChat() {
  const [chats, setChats] = useState<EphemeralChat[]>([]);
  const [activeChatId, setActiveChatId] = useState<string | null>(null);

  const createChat = useCallback(async (title: string): Promise<string> => {
    const conv = await api.createEphemeralConversation(title);
    const newChat: EphemeralChat = { id: conv.id, title, messages: [], loading: false };
    setChats(prev => [...prev, newChat]);
    setActiveChatId(conv.id);
    return conv.id;
  }, []);

  const sendMessage = useCallback(async (chatId: string, content: string) => {
    const optimistic: Message = {
      id: 'temp-' + Date.now(),
      conversation_id: chatId,
      role: 'user',
      content,
      created_at: new Date().toISOString(),
    };
    setChats(prev => prev.map(c =>
      c.id === chatId ? { ...c, messages: [...c.messages, optimistic] } : c
    ));
    try {
      const msg = await api.createMessage(chatId, { role: 'user', content });
      setChats(prev => prev.map(c =>
        c.id === chatId ? {
          ...c,
          messages: c.messages.map(m => m.id === optimistic.id ? msg : m),
        } : c
      ));
    } catch {
      setChats(prev => prev.map(c =>
        c.id === chatId ? {
          ...c,
          messages: c.messages.filter(m => m.id !== optimistic.id),
        } : c
      ));
    }
  }, []);

  const closeChat = useCallback(async (chatId: string) => {
    try {
      await api.deleteConversation(chatId);
    } catch { /* ignore */ }
    setChats(prev => prev.filter(c => c.id !== chatId));
    setActiveChatId(prev => prev === chatId ? null : prev);
  }, []);

  const selectChat = useCallback((chatId: string | null) => {
    setActiveChatId(chatId);
  }, []);

  return { chats, activeChatId, createChat, sendMessage, closeChat, selectChat };
}