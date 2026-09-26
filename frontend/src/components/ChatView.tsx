import React, { useRef, useEffect } from 'react';
import type { Message } from '../types';
import { MessageBubble } from './MessageBubble';
import { MessageInput, type MessageInputHandle } from './MessageInput';

interface ChatViewProps {
  messages: Message[];
  loading: boolean;
  onSendMessage: (content: string) => void;
  streaming?: boolean;
  streamingContent?: string;
  activeTools?: string[];
  settings?: Record<string, string> | null;
}

export const ChatView: React.FC<ChatViewProps> = (props) => {
  const { messages, loading, onSendMessage, streaming, streamingContent, activeTools } = props;
  const bottomRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<MessageInputHandle>(null);

  const allMessages = streaming && streamingContent
    ? [...messages, {
        id: 'streaming',
        role: 'assistant' as const,
        content: streamingContent,
        created_at: new Date().toISOString(),
      }]
    : messages;

  // Auto-scroll to bottom when messages or streaming content changes
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [allMessages, streamingContent]);

  // Focus input when streaming ends
  useEffect(() => {
    if (!streaming) {
      inputRef.current?.focus();
    }
  }, [streaming]);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: 'calc(100vh - 64px)' }}>
      <div style={{ flex: 1, overflow: 'auto', padding: '16px 0', scrollBehavior: 'smooth' }}>
        {loading && <div>Cargando...</div>}
        {allMessages.map(msg => (
          <MessageBubble key={msg.id} message={msg} />
        ))}
        <div ref={bottomRef} />
        {allMessages.length === 0 && !loading && (
          <div style={{ textAlign: 'center', marginTop: '40vh', color: 'rgba(255,255,255,0.45)' }}>
            Inicia una conversación con Alfred
          </div>
        )}
      </div>
      {activeTools && activeTools.length > 0 && (
        <div style={{
          padding: '8px 24px',
          background: 'rgba(255, 255, 255, 0.03)',
          borderTop: '1px solid rgba(255,255,255,0.1)',
          color: 'rgba(255,255,255,0.65)',
          fontSize: 13,
          display: 'flex',
          gap: 8,
          alignItems: 'center',
        }}>
          <span>🔧</span>
          {activeTools.map((tool, i) => (
            <span key={i}>
              {i > 0 && <span style={{ margin: '0 4px' }}>·</span>}
              <code style={{
                background: 'rgba(255,255,255,0.08)',
                padding: '2px 6px',
                borderRadius: 4,
                fontSize: 12,
              }}>
                {tool}
              </code>
            </span>
          ))}
          <span style={{ marginLeft: 'auto', fontSize: 11 }}>ejecutando...</span>
        </div>
      )}
      <div style={{ padding: 16, borderTop: '1px solid rgba(255,255,255,0.1)' }}>
        <MessageInput ref={inputRef} onSend={onSendMessage} disabled={loading || !!streaming} />
      </div>
    </div>
  );
};