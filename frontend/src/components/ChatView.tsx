import React from 'react';
import { Typography } from 'antd';
import type { Message } from '../types';
import { MessageBubble } from './MessageBubble';
import { MessageInput } from './MessageInput';

const { Title } = Typography;

interface ChatViewProps {
  messages: Message[];
  loading: boolean;
  hasMore: boolean;
  onLoadMore: () => void;
  onSendMessage: (content: string) => void;
  title: string;
  streaming?: boolean;
  streamingContent?: string;
  activeTools?: string[];
}

export const ChatView: React.FC<ChatViewProps> = (props) => {
  const { messages, loading, onSendMessage, title, streaming, streamingContent, activeTools } = props;

  const allMessages = streaming && streamingContent
    ? [...messages, {
        id: 'streaming',
        conversation_id: '',
        role: 'assistant' as const,
        content: streamingContent,
        created_at: new Date().toISOString(),
      }]
    : messages;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div style={{ padding: '16px 24px', borderBottom: '1px solid rgba(255,255,255,0.1)' }}>
        <Title level={4} style={{ margin: 0 }}>{title}</Title>
      </div>
      <div style={{ flex: 1, overflow: 'auto', padding: 24 }}>
        {loading && <div>Cargando...</div>}
        {allMessages.map(msg => (
          <MessageBubble key={msg.id} message={msg} />
        ))}
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
        <MessageInput onSend={onSendMessage} disabled={loading || !!streaming} />
      </div>
    </div>
  );
};