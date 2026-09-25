import React, { useState } from 'react';
import { Layout, Typography, Button, Space } from 'antd';
import { UserOutlined, SettingOutlined } from '@ant-design/icons';
import { ChatView } from './ChatView';
import { ProfileEditor } from './ProfileEditor';
import { SettingsEditor } from './SettingsEditor';
import { useMainChat } from '../hooks/useMainChat';
import { useProfile } from '../hooks/useProfile';
import { useSettings } from '../hooks/useSettings';

const { Header, Content } = Layout;
const { Text } = Typography;

export const AppLayout: React.FC = () => {
  const mainChat = useMainChat();
  const profile = useProfile();
  const { settings } = useSettings();
  const [profileVisible, setProfileVisible] = useState(false);
  const [settingsVisible, setSettingsVisible] = useState(false);

  return (
    <Layout style={{ minHeight: '100vh' }}>
      <Header style={{
        display: 'flex', alignItems: 'center', justifyContent: 'space-between',
        padding: '0 24px', background: '#1a1a2e', borderBottom: '1px solid rgba(255,255,255,0.1)'
      }}>
        <Text strong style={{ color: '#fff', fontSize: 18 }}>💬 Alfred</Text>
        <Space>
          <Button type="text" icon={<UserOutlined />} onClick={() => setProfileVisible(true)} style={{ color: 'rgba(255,255,255,0.65)' }} />
          <Button type="text" icon={<SettingOutlined />} onClick={() => setSettingsVisible(true)} style={{ color: 'rgba(255,255,255,0.65)' }} />
        </Space>
      </Header>
      <Content style={{ padding: 0, background: '#000', height: 'calc(100vh - 64px)', overflow: 'hidden' }}>
        <ChatView
          messages={mainChat.messages}
          loading={mainChat.loading}
          onSendMessage={mainChat.sendMessage}
          streaming={mainChat.streaming}
          streamingContent={mainChat.streamingContent}
          activeTools={mainChat.activeTools}
          settings={settings}
        />
      </Content>
      <ProfileEditor profile={profile.profile} onUpdate={profile.updateProfile} visible={profileVisible} onClose={() => setProfileVisible(false)} />
      <SettingsEditor visible={settingsVisible} onClose={() => setSettingsVisible(false)} />
    </Layout>
  );
};