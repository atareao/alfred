import React, { useState, useEffect } from "react";
import {
  Modal,
  Tabs,
  Form,
  Input,
  InputNumber,
  Button,
  message,
  Space,
  Spin,
} from "antd";
import { useProfile } from "../hooks/useProfile";
import { useSettings } from "../hooks/useSettings";

const { TextArea } = Input;

export interface SettingsDialogProps {
  visible: boolean;
  onClose: () => void;
}

export interface SettingsFormValues {
  font_size: number;
  max_window_tokens: number;
  system_prompt: string;
  archivist_prompt: string;
  collapse_prompt: string;
  message_page_size: number;
  openweather_api_key: string;
  google_places_api_key: string;
  brave_search_api_key: string;
}

export const SettingsDialog: React.FC<SettingsDialogProps> = ({
  visible,
  onClose,
}) => {
  const { profile, updateProfile } = useProfile();
  const {
    settings,
    loading: settingsLoading,
    saving,
    updateSettings,
    resetToDefaults,
  } = useSettings();

  const [profileForm] = Form.useForm();
  const [settingsForm] = Form.useForm();
  const [resetting, setResetting] = useState(false);
  const [profileSaving, setProfileSaving] = useState(false);

  // Load settings into form when visible changes
  useEffect(() => {
    if (settings && visible) {
      settingsForm.setFieldsValue({
        font_size: parseInt(settings.font_size || "16"),
        max_window_tokens: parseInt(settings.max_window_tokens || "10000"),
        system_prompt: settings.system_prompt || "",
        archivist_prompt: settings.archivist_prompt || "",
        collapse_prompt: settings.collapse_prompt || "",
        message_page_size: parseInt(settings.message_page_size || "50"),
        openweather_api_key: settings.openweather_api_key || "",
        google_places_api_key: settings.google_places_api_key || "",
        brave_search_api_key: settings.brave_search_api_key || "",
      });
    }
  }, [settings, visible, settingsForm]);

  // Set profile form values when visible changes
  useEffect(() => {
    if (profile && visible) {
      profileForm.setFieldsValue({
        name: profile.name || "",
        avatar_url: profile.avatar_url || "",
      });
    }
  }, [profile, visible, profileForm]);

  const handleProfileSubmit = async (values: {
    name: string;
    avatar_url: string;
  }) => {
    setProfileSaving(true);
    try {
      await updateProfile({
        name: values.name,
        avatar_url: values.avatar_url,
      });
      message.success("Perfil actualizado");
      onClose();
    } catch {
      message.error("Error al actualizar perfil");
    } finally {
      setProfileSaving(false);
    }
  };

  const handleSettingsSubmit = async (
    values: Partial<SettingsFormValues>,
  ) => {
    try {
      await updateSettings({
        font_size: (
          values.font_size ?? parseInt(settings?.font_size || "16")
        ).toString(),
        max_window_tokens: (
          values.max_window_tokens ?? parseInt(settings?.max_window_tokens || "10000")
        ).toString(),
        system_prompt: values.system_prompt ?? settings?.system_prompt ?? "",
        archivist_prompt: values.archivist_prompt ?? settings?.archivist_prompt ?? "",
        collapse_prompt: values.collapse_prompt ?? settings?.collapse_prompt ?? "",
        message_page_size: (
          values.message_page_size ?? parseInt(settings?.message_page_size || "50")
        ).toString(),
        openweather_api_key: values.openweather_api_key ?? settings?.openweather_api_key ?? "",
        google_places_api_key: values.google_places_api_key ?? settings?.google_places_api_key ?? "",
        brave_search_api_key: values.brave_search_api_key ?? settings?.brave_search_api_key ?? "",
      });
      message.success("Ajustes guardados");
      onClose();
    } catch {
      message.error("Error al guardar ajustes");
    }
  };

  const handleReset = async () => {
    setResetting(true);
    try {
      await resetToDefaults();
      settingsForm.setFieldsValue({
        font_size: 16,
        max_window_tokens: 10000,
        message_page_size: 50,
        openweather_api_key: "",
        google_places_api_key: "",
        brave_search_api_key: "",
      });
      message.success("Valores por defecto restaurados");
    } catch {
      message.error("Error al restaurar valores");
    } finally {
      setResetting(false);
    }
  };

  const renderSettingsLoading = () => (
    <Spin style={{ display: "flex", justifyContent: "center", margin: "24px 0" }} />
  );

  return (
    <Modal
      title={
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
          }}
        >
          <span>⚙️ Settings</span>
          <Button type="text" aria-label="Close" onClick={onClose} danger>
            ✕
          </Button>
        </div>
      }
      open={visible}
      onCancel={onClose}
      closable={false}
      footer={null}
      width={600}
    >
      <Tabs
        items={[
          {
            key: "profile",
            label: "Perfil",
            children: (
              <Form
                form={profileForm}
                layout="vertical"
                onFinish={handleProfileSubmit}
                initialValues={{ name: "", avatar_url: "" }}
              >
                <Form.Item
                  label="Nombre"
                  name="name"
                >
                  <Input />
                </Form.Item>
                <Form.Item
                  label="Avatar URL"
                  name="avatar_url"
                >
                  <Input />
                </Form.Item>
                <Button type="primary" htmlType="submit" loading={profileSaving}>
                  Guardar
                </Button>
              </Form>
            ),
          },
          {
            key: "interface",
            label: "Interfaz",
            children: settingsLoading ? (
              renderSettingsLoading()
            ) : (
              <Form
                form={settingsForm}
                layout="vertical"
                onFinish={handleSettingsSubmit}
              >
                <Form.Item
                  label="Tamaño de fuente"
                  name="font_size"
                >
                  <InputNumber
                    min={12}
                    max={24}
                    step={1}
                    style={{ width: "100%" }}
                  />
                </Form.Item>
                <Form.Item
                  label="Ventana de contexto (tokens)"
                  name="max_window_tokens"
                >
                  <InputNumber
                    min={1000}
                    max={100000}
                    step={1000}
                    style={{ width: "100%" }}
                  />
                </Form.Item>
                <Form.Item
                  label="Tamaño de página"
                  name="message_page_size"
                >
                  <InputNumber
                    min={10}
                    max={100}
                    step={10}
                    style={{ width: "100%" }}
                  />
                </Form.Item>
                <Space>
                  <Button type="primary" htmlType="submit" loading={saving}>
                    Guardar
                  </Button>
                  <Button onClick={handleReset} loading={resetting} danger>
                    Restaurar valores por defecto
                  </Button>
                </Space>
              </Form>
            ),
          },
          {
            key: "prompts",
            label: "Prompts",
            children: settingsLoading ? (
              renderSettingsLoading()
            ) : (
              <Form
                form={settingsForm}
                layout="vertical"
                onFinish={handleSettingsSubmit}
              >
                <Tabs
                  items={[
                    {
                      key: "system",
                      label: "System",
                      forceRender: true,
                      children: (
                        <Form.Item
                          label="System Prompt"
                          name="system_prompt"
                        >
                          <TextArea rows={10} />
                        </Form.Item>
                      ),
                    },
                    {
                      key: "archivist",
                      label: "Archivist",
                      forceRender: true,
                      children: (
                        <Form.Item
                          label="Archivist Prompt"
                          name="archivist_prompt"
                        >
                          <TextArea rows={10} />
                        </Form.Item>
                      ),
                    },
                    {
                      key: "collapse",
                      label: "Collapse",
                      forceRender: true,
                      children: (
                        <Form.Item
                          label="Collapse Prompt"
                          name="collapse_prompt"
                        >
                          <TextArea rows={10} />
                        </Form.Item>
                      ),
                    },
                  ]}
                />
                <Button type="primary" htmlType="submit" loading={saving}>
                  Guardar
                </Button>
              </Form>
            ),
          },
          {
            key: "api-keys",
            label: "API Keys",
            children: settingsLoading ? (
              renderSettingsLoading()
            ) : (
              <Form
                form={settingsForm}
                layout="vertical"
                onFinish={handleSettingsSubmit}
              >
                <Form.Item
                  label="OpenWeatherMap API Key"
                  name="openweather_api_key"
                >
                  <Input.Password placeholder="Dejar vacío para usar variable de entorno" />
                </Form.Item>
                <Form.Item
                  label="Google Places API Key"
                  name="google_places_api_key"
                >
                  <Input.Password placeholder="Dejar vacío para usar variable de entorno" />
                </Form.Item>
                <Form.Item
                  label="Brave Search API Key"
                  name="brave_search_api_key"
                >
                  <Input.Password placeholder="Dejar vacío para usar variable de entorno" />
                </Form.Item>
                <Button type="primary" htmlType="submit" loading={saving}>
                  Guardar
                </Button>
              </Form>
            ),
          },
        ]}
      />
    </Modal>
  );
};