import React from 'react';
import { Typography } from 'antd';
import { UserOutlined, RobotOutlined, InfoCircleOutlined, CodeOutlined } from '@ant-design/icons';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import type { Message } from '../types';

const { Text } = Typography;

interface MessageBubbleProps {
  message: Message;
}

export const MessageBubble: React.FC<MessageBubbleProps> = ({ message }) => {
  const config = getRoleConfig(message.role);

  const renderContent = () => {
    if (message.role !== 'assistant') {
      return <Text style={{ color: config.color }}>{message.content}</Text>;
    }

    // Separar footer de tools del contenido principal
    const footerMatch = message.content.match(/\n\n---\n🔧(.+)$/s);
    const mainContent = footerMatch ? message.content.slice(0, footerMatch.index) : message.content;
    const toolsFooter = footerMatch ? footerMatch[1].trim() : null;

    return (
      <div>
        <div style={{ overflowX: 'auto' }}>
          <ReactMarkdown remarkPlugins={[remarkGfm]}>{mainContent}</ReactMarkdown>
        </div>
        {toolsFooter && (
          <div style={{
            marginTop: 12,
            paddingTop: 8,
            borderTop: '1px solid rgba(255,255,255,0.15)',
            fontSize: 12,
            color: 'rgba(255,255,255,0.5)',
            display: 'flex',
            gap: 4,
            alignItems: 'center',
          }}>
            <span>🔧</span>
            <span>{toolsFooter}</span>
          </div>
        )}
      </div>
    );
  };

  return (
    <div style={{
      display: 'flex',
      justifyContent: config.justifyContent,
      marginBottom: 12,
      fontStyle: config.fontStyle,
      fontFamily: config.fontFamily,
    }}>
      {config.showIcon && config.iconPosition === 'left' && (
        <div style={{ marginRight: 8, marginTop: 8 }}>
          {config.icon}
        </div>
      )}
      <div style={{
        maxWidth: '70%',
        padding: '10px 16px',
        borderRadius: 12,
        background: config.background,
        color: config.color,
        textAlign: config.textAlign as 'left' | 'center' | 'right',
      }}>
        {(message.role === 'assistant' || message.role === 'tool') ? (
          <div style={{ color: config.color, overflowX: 'auto' }}>
            <ReactMarkdown remarkPlugins={[remarkGfm]}>{message.content}</ReactMarkdown>
          </div>
        ) : (
          renderContent()
        )}
        <div style={{ fontSize: 11, marginTop: 4, opacity: 0.5 }}>
          {message.role}
        </div>
      </div>
      {config.showIcon && config.iconPosition === 'right' && (
        <div style={{ marginLeft: 8, marginTop: 8 }}>
          {config.icon}
        </div>
      )}
    </div>
  );
};

interface RoleConfig {
  justifyContent: string;
  background: string;
  color: string;
  fontStyle: string;
  fontFamily: string;
  textAlign: string;
  showIcon: boolean;
  iconPosition: 'left' | 'right';
  icon: React.ReactNode;
}

function getRoleConfig(role: string): RoleConfig {
  switch (role) {
    case 'user':
      return {
        justifyContent: 'flex-end',
        background: '#1677ff',
        color: '#fff',
        fontStyle: 'normal',
        fontFamily: 'inherit',
        textAlign: 'left',
        showIcon: true,
        iconPosition: 'right' as const,
        icon: <UserOutlined style={{ color: '#1677ff' }} />,
      };
    case 'assistant':
      return {
        justifyContent: 'flex-start',
        background: 'rgba(255,255,255,0.06)',
        color: '#fff',
        fontStyle: 'normal',
        fontFamily: 'inherit',
        textAlign: 'left',
        showIcon: true,
        iconPosition: 'left' as const,
        icon: <RobotOutlined style={{ color: '#52c41a' }} />,
      };
    case 'system':
      return {
        justifyContent: 'center',
        background: 'transparent',
        color: 'rgba(255,255,255,0.45)',
        fontStyle: 'italic',
        fontFamily: 'inherit',
        textAlign: 'center',
        showIcon: true,
        iconPosition: 'left' as const,
        icon: <InfoCircleOutlined style={{ color: 'rgba(255,255,255,0.45)' }} />,
      };
    case 'tool':
      return {
        justifyContent: 'flex-start',
        background: 'rgba(255,255,255,0.03)',
        color: 'rgba(255,255,255,0.75)',
        fontStyle: 'normal',
        fontFamily: 'monospace',
        textAlign: 'left',
        showIcon: true,
        iconPosition: 'left' as const,
        icon: <CodeOutlined style={{ color: 'rgba(255,255,255,0.45)' }} />,
      };
    default:
      return {
        justifyContent: 'flex-start',
        background: 'rgba(255,255,255,0.06)',
        color: '#fff',
        fontStyle: 'normal',
        fontFamily: 'inherit',
        textAlign: 'left',
        showIcon: false,
        iconPosition: 'left' as const,
        icon: null,
      };
  }
}