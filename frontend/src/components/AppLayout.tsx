import React, { useState, useEffect } from "react";
import { Layout, Typography, Button, Space, Modal } from "antd";
import {
  UserOutlined,
  SettingOutlined,
  CalendarOutlined,
  CheckSquareOutlined,
  BarChartOutlined,
} from "@ant-design/icons";
import { ChatView } from "./ChatView";
import { ProfileEditor } from "./ProfileEditor";
import { SettingsEditor } from "./SettingsEditor";
import { CalendarView } from "./CalendarView";
import { TaskView } from "./TaskView";
import { useMainChat } from "../hooks/useMainChat";
import { useProfile } from "../hooks/useProfile";
import { useSettings } from "../hooks/useSettings";
import { useNavigate } from "react-router-dom";

const { Header, Content } = Layout;
const { Text } = Typography;

interface AppLayoutProps {
  children?: React.ReactNode;
}

export const AppLayout: React.FC<AppLayoutProps> = ({ children }) => {
  const mainChat = useMainChat();
  const profile = useProfile();
  const { settings } = useSettings();
  const navigate = useNavigate();

  // Apply font-size as CSS variable on root element
  const fontSize = settings?.font_size ? parseInt(settings.font_size, 10) : 16;
  useEffect(() => {
    document.documentElement.style.setProperty(
      "--font-size-base",
      `${fontSize}px`,
    );
  }, [fontSize]);
  const [profileVisible, setProfileVisible] = useState(false);
  const [settingsVisible, setSettingsVisible] = useState(false);
  const [calendarVisible, setCalendarVisible] = useState(false);
  const [tasksVisible, setTasksVisible] = useState(false);

  return (
    <Layout style={{ minHeight: "100vh" }}>
      <Header
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "0 24px",
          background: "#1a1a2e",
          borderBottom: "1px solid rgba(255,255,255,0.1)",
        }}
      >
        <Text strong style={{ color: "#fff", fontSize: 18 }}>
          💬 Alfred
        </Text>
        <Space>
          <Button
            type="text"
            icon={<CheckSquareOutlined />}
            onClick={() => setTasksVisible(true)}
            style={{ color: "rgba(255,255,255,0.65)" }}
          />
          <Button
            type="text"
            icon={<CalendarOutlined />}
            onClick={() => setCalendarVisible(true)}
            style={{ color: "rgba(255,255,255,0.65)" }}
          />
          <Button
            type="text"
            icon={<BarChartOutlined />}
            onClick={() => navigate("/stats")}
            style={{ color: "rgba(255,255,255,0.65)" }}
          />
          <Button
            type="text"
            icon={<UserOutlined />}
            onClick={() => setProfileVisible(true)}
            style={{ color: "rgba(255,255,255,0.65)" }}
          />
          <Button
            type="text"
            icon={<SettingOutlined />}
            onClick={() => setSettingsVisible(true)}
            style={{ color: "rgba(255,255,255,0.65)" }}
          />
        </Space>
      </Header>
      <Content
        style={{
          padding: 0,
          background: "#000",
          height: "calc(100vh - 64px)",
          overflow: "hidden",
        }}
      >
        {children || (
          <ChatView
            messages={mainChat.messages}
            loading={mainChat.loading}
            onSendMessage={mainChat.sendMessage}
            streaming={mainChat.streaming}
            streamingContent={mainChat.streamingContent}
            activeTools={mainChat.activeTools}
            settings={settings}
          />
        )}
      </Content>
      <Modal
        title="📅 Agenda"
        open={calendarVisible}
        onCancel={() => setCalendarVisible(false)}
        footer={null}
        width={900}
      >
        <CalendarView />
      </Modal>
      <Modal
        title="✅ Tasks"
        open={tasksVisible}
        onCancel={() => setTasksVisible(false)}
        footer={null}
        width={1000}
      >
        <TaskView onClose={() => setTasksVisible(false)} />
      </Modal>
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
