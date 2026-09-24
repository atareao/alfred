import React, { useState } from 'react';
import { Input, Button } from 'antd';
import { SendOutlined, LoadingOutlined } from '@ant-design/icons';

const { TextArea } = Input;

interface MessageInputProps {
  onSend: (content: string) => void;
  disabled: boolean;
}

export const MessageInput: React.FC<MessageInputProps> = ({ onSend, disabled }) => {
  const [value, setValue] = useState('');

  const handleSend = () => {
    if (value.trim() && !disabled) {
      onSend(value.trim());
      setValue('');
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  return (
    <div style={{ display: 'flex', gap: 8, alignItems: 'flex-end' }}>
      <TextArea
        value={value}
        onChange={e => setValue(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder="Escribe un mensaje a Alfred..."
        autoSize={{ minRows: 2, maxRows: 6 }}
        disabled={disabled}
        style={{ flex: 1 }}
      />
      <Button
        type="primary"
        icon={disabled ? <LoadingOutlined /> : <SendOutlined />}
        onClick={handleSend}
        disabled={disabled || !value.trim()}
        loading={disabled}
      />
    </div>
  );
};