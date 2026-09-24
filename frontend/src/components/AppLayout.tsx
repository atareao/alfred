import React, { useState } from 'react';
import { Layout, Menu, Typography, Button } from 'antd';
import {
  CommentOutlined,
  UserOutlined,
  PlusOutlined,
  CloseOutlined,
  SettingOutlined,
  MenuFoldOutlined,
  MenuUnfoldOutlined,
} from '@ant-design/icons';
import { ChatView } from './ChatView';
import { ProfileEditor } from './ProfileEditor';
import { SettingsEditor } from './SettingsEditor';
import { useMainChat } from '../hooks/useMainChat';
import { useEphemeralChat } from '../hooks/useEphemeralChat';
import { useProfile } from '../hooks/useProfile';

const { Sider, Content } = Layout;
const { Text } = Typography;

export const AppLayout: React.FC = () => {
  const mainChat = useMainChat();
  const ephemeral = useEphemeralChat();
  const profile = useProfile();
  const [profileVisible, setProfileVisible] = useState(false);
  const [settingsVisible, setSettingsVisible] = useState(false);
  const [collapsed, setCollapsed] = useState(false);

  const activeTitle = ephemeral.activeChatId
    ? ephemeral.chats.find(c => c.id === ephemeral.activeChatId)?.title || 'Chat'
    : '💬 Alfred';

  const chatItems = [
    {
      key: 'main',
      icon: <CommentOutlined />,
      label: 'Alfred',
    },
    ...ephemeral.chats.map(chat => ({
      key: chat.id,
      icon: <PlusOutlined />,
      label: collapsed ? null : (
        <span>
          {chat.title}
          <Button
            type="text"
            size="small"
            icon={<CloseOutlined />}
            onClick={(e) => { e.stopPropagation(); ephemeral.closeChat(chat.id); }}
            style={{ float: 'right', color: 'rgba(255,255,255,0.45)' }}
          />
        </span>
      ),
    })),
  ];

  const bottomItems = [
    { type: 'divider' as const },
    {
      key: 'profile',
      icon: <UserOutlined />,
      label: collapsed ? null : 'Perfil',
      onClick: () => setProfileVisible(true),
    },
    {
      key: 'settings',
      icon: <SettingOutlined />,
      label: collapsed ? null : 'Ajustes',
      onClick: () => setSettingsVisible(true),
    },
    {
      key: 'collapse',
      icon: collapsed ? <MenuUnfoldOutlined /> : <MenuFoldOutlined />,
      label: collapsed ? null : 'Contraer',
      onClick: () => setCollapsed(!collapsed),
    },
  ];

  return (
    <Layout style={{ minHeight: '100vh' }}>
      <Sider
        width={280}
        theme="dark"
        collapsible
        collapsed={collapsed}
        onCollapse={setCollapsed}
        trigger={null}
      >
        <div style={{ padding: '16px', textAlign: 'center' }}>
          <Text strong style={{ color: '#fff', fontSize: 18 }}>
            {collapsed ? 'A' : 'Alfred'}
          </Text>
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', height: 'calc(100% - 56px)' }}>
          <div style={{ flex: 1, overflow: 'auto' }}>
            <Menu
              theme="dark"
              mode="inline"
              selectedKeys={[ephemeral.activeChatId || 'main']}
              onClick={({ key }) => {
                if (key !== 'profile' && key !== 'settings' && key !== 'collapse') {
                  ephemeral.selectChat(key === 'main' ? null : key);
                }
              }}
              items={chatItems}
            />
          </div>

          <Menu
            theme="dark"
            mode="inline"
            selectable={false}
            onClick={({ key }) => {
              if (key === 'profile') setProfileVisible(true);
              else if (key === 'settings') setSettingsVisible(true);
              else if (key === 'collapse') setCollapsed(!collapsed);
            }}
            items={bottomItems}
            style={{ borderInlineEnd: 'none' }}
          />
        </div>
      </Sider>

      <Layout>
        <Content style={{ display: 'flex', flexDirection: 'column' }}>
          <ChatView
            messages={mainChat.messages}
            loading={mainChat.loading}
            hasMore={mainChat.hasMore}
            onLoadMore={mainChat.loadMore}
            onSendMessage={mainChat.sendMessage}
            title={activeTitle}
            streaming={mainChat.streaming}
            streamingContent={mainChat.streamingContent}
            activeTools={mainChat.activeTools}
          />
        </Content>
      </Layout>

      <ProfileEditor
        profile={profile.profile}
        onUpdate={profile.updateProfile}
        visible={profileVisible}
        onClose={() => setProfileVisible(false)}
      />
      <SettingsEditor
        visible={settingsVisible}
        onClose={() => setSettingsVisible(false)}
      />
    </Layout>
  );
};