import React, { useState, useEffect } from 'react';
import { Layout, Typography, Button, Space, Modal } from 'antd';
import { UserOutlined, SettingOutlined, CalendarOutlined } from '@ant-design/icons';
import { ChatView } from './ChatView';
import { ProfileEditor } from './ProfileEditor';
import { SettingsEditor } from './SettingsEditor';
import { CalendarView } from './CalendarView';
import { useMainChat } from '../hooks/useMainChat';
import { useProfile } from '../hooks/useProfile';
import { useSettings } from '../hooks/useSettings';

const { Header, Content } = Layout;
const { Text } = Typography;

export const AppLayout: React.FC = () => {
  const mainChat = useMainChat();
  const profile = useProfile();
  const { settings } = useSettings();

  // Apply font-size as CSS variable on root element
  const fontSize = settings?.font_size ? parseInt(settings.font_size, 10) : 16;
  useEffect(() => {
    document.documentElement.style.setProperty('--font-size-base', `${fontSize}px`);
  }, [fontSize]);
  const [profileVisible, setProfileVisible] = useState(false);
  const [settingsVisible, setSettingsVisible] = useState(false);
  const [calendarVisible, setCalendarVisible] = useState(false);

  return (
    <Layout style={{ minHeight: '100vh' }}>
      <Header style={{
        display: 'flex', alignItems: 'center', justifyContent: 'space-between',
        padding: '0 24px', background: '#1a1a2e', borderBottom: '1px solid rgba(255,255,255,0.1)'
      }}>
        <Text strong style={{ color: '#fff', fontSize: 18 }}>💬 Alfred</Text>
        <Space>
          <Button type="text" icon={<CalendarOutlined />} onClick={() => setCalendarVisible(true)} style={{ color: 'rgba(255,255,255,0.65)' }} />
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
      <Modal title="📅 Agenda" open={calendarVisible} onCancel={() => setCalendarVisible(false)} footer={null} width={900}>
        <CalendarView />
      </Modal>
      <ProfileEditor profile={profile.profile} onUpdate={profile.updateProfile} visible={profileVisible} onClose={() => setProfileVisible(false)} />
      <SettingsEditor visible={settingsVisible} onClose={() => setSettingsVisible(false)} />
    </Layout>
  );
};