import React, { useState, useEffect } from "react";
import {
  Modal,
  Tabs,
  Form,
  Input,
  InputNumber,
  Button,
  App as AntdApp,
  Space,
  Spin,
  Alert,
} from "antd";
import { useSettings } from "../hooks/useSettings";
import { useProfileContext } from "../contexts/ProfileContext";
import { PersistentMemoryPanel } from "./PersistentMemoryPanel";

const { TextArea } = Input;

export interface SettingsDialogProps {
  visible: boolean;
  onClose: () => void;
}

const URL_SCHEME_RE = /^[a-zA-Z][a-zA-Z0-9+.-]*:/;

function isAllowedAvatarUrl(value: string | null | undefined): boolean {
  const trimmed = (value ?? "").trim();
  // Vacío o solo espacios: válido.
  if (trimmed === "") return true;
  // Espacios o caracteres de control embebidos: inválido.
  if (/[\s\u0000-\u001f]/.test(trimmed)) return false;
  // URL relativa al protocolo (//host): apunta a un host externo, inválida.
  if (trimmed.startsWith("//")) return false;
  // Ruta relativa del propio host: válida.
  if (trimmed.startsWith("/")) return true;
  // URL absoluta http/https (case-insensitive): válida.
  if (/^https?:\/\//i.test(trimmed)) return true;
  // Sin esquema (p. ej. "example.com/a.png"): se trata como ruta relativa.
  if (!URL_SCHEME_RE.test(trimmed)) return true;
  // Cualquier otro esquema (javascript:, data:, file:, ftp:, C:, …): inválido.
  return false;
}

const CONSOLIDATOR_PLACEHOLDERS = [
  "{{ ESTADO_ACTUAL }}",
  "{{ BLOQUE_DE_MENSAJES }}",
] as const;

function getMissingConsolidatorPlaceholders(
  value: string | undefined,
): string[] {
  const text = value ?? "";
  return CONSOLIDATOR_PLACEHOLDERS.filter((placeholder) =>
    !text.includes(placeholder),
  );
}

export interface SettingsFormValues {
  font_size: number;
  max_window_tokens: number;
  system_prompt: string;
  archivist_prompt: string;
  collapse_prompt: string;
  consolidator_prompt: string;
  message_page_size: number;
  openweather_api_key: string;
  google_places_api_key: string;
  brave_search_api_key: string;
  MEMORY_HALF_LIFE_DAYS: number;
  SIMILARITY_THRESHOLD: number;
  RAG_BUDGET_TOKENS: number;
  MEMORY_KNN_CANDIDATES: number;
}

export const SettingsDialog: React.FC<SettingsDialogProps> = ({
  visible,
  onClose,
}) => {
  const { message: messageApi } = AntdApp.useApp();
  const { profile, updateProfile } = useProfileContext();
  const {
    settings,
    loading: settingsLoading,
    saving,
    updateSettings,
    resetToDefaults,
  } = useSettings();

  const [profileForm] = Form.useForm();
  const [settingsForm] = Form.useForm();
  const consolidatorPrompt = Form.useWatch("consolidator_prompt", settingsForm);
  const missingConsolidatorPlaceholders =
    getMissingConsolidatorPlaceholders(consolidatorPrompt);
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
        consolidator_prompt: settings.consolidator_prompt || "",
        message_page_size: parseInt(settings.message_page_size || "50"),
        openweather_api_key: settings.openweather_api_key || "",
        google_places_api_key: settings.google_places_api_key || "",
        brave_search_api_key: settings.brave_search_api_key || "",
        MEMORY_HALF_LIFE_DAYS: parseFloat(
          settings.MEMORY_HALF_LIFE_DAYS || "90",
        ),
        SIMILARITY_THRESHOLD: parseFloat(
          settings.SIMILARITY_THRESHOLD || "0.5",
        ),
        RAG_BUDGET_TOKENS: parseInt(settings.RAG_BUDGET_TOKENS || "800"),
        MEMORY_KNN_CANDIDATES: parseInt(
          settings.MEMORY_KNN_CANDIDATES || "20",
        ),
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
        avatar_url: (values.avatar_url ?? "").trim(),
      });
      messageApi.success("Perfil actualizado");
      onClose();
    } catch {
      messageApi.error("Error al actualizar perfil");
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
        consolidator_prompt:
          values.consolidator_prompt ?? settings?.consolidator_prompt ?? "",
        message_page_size: (
          values.message_page_size ?? parseInt(settings?.message_page_size || "50")
        ).toString(),
        openweather_api_key: values.openweather_api_key ?? settings?.openweather_api_key ?? "",
        google_places_api_key: values.google_places_api_key ?? settings?.google_places_api_key ?? "",
        brave_search_api_key: values.brave_search_api_key ?? settings?.brave_search_api_key ?? "",
        MEMORY_HALF_LIFE_DAYS: (
          values.MEMORY_HALF_LIFE_DAYS ??
          parseFloat(settings?.MEMORY_HALF_LIFE_DAYS || "90")
        ).toString(),
        SIMILARITY_THRESHOLD: (
          values.SIMILARITY_THRESHOLD ??
          parseFloat(settings?.SIMILARITY_THRESHOLD || "0.5")
        ).toString(),
        RAG_BUDGET_TOKENS: (
          values.RAG_BUDGET_TOKENS ??
          parseInt(settings?.RAG_BUDGET_TOKENS || "800")
        ).toString(),
        MEMORY_KNN_CANDIDATES: (
          values.MEMORY_KNN_CANDIDATES ??
          parseInt(settings?.MEMORY_KNN_CANDIDATES || "20")
        ).toString(),
      });
      messageApi.success("Ajustes guardados");
      onClose();
    } catch {
      messageApi.error("Error al guardar ajustes");
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
      messageApi.success("Valores por defecto restaurados");
    } catch {
      messageApi.error("Error al restaurar valores");
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
                  rules={[
                    {
                      validator: (_rule, value: string) =>
                        isAllowedAvatarUrl(value)
                          ? Promise.resolve()
                          : Promise.reject(
                              new Error(
                                "Usa una URL http(s) o una ruta relativa",
                              ),
                            ),
                    },
                  ]}
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
                    {
                      key: "consolidator",
                      label: "Consolidator",
                      forceRender: true,
                      children: (
                        <Form.Item
                          label="Consolidator Prompt"
                          name="consolidator_prompt"
                        >
                          <TextArea rows={10} />
                        </Form.Item>
                      ),
                    },
                  ]}
                />
                {missingConsolidatorPlaceholders.length > 0 && (
                  <Alert
                    type="warning"
                    showIcon
                    style={{ marginBottom: 16 }}
                    message={`Faltan placeholders en el prompt del consolidador: ${missingConsolidatorPlaceholders.join(", ")}`}
                  />
                )}
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
          {
            key: "memory",
            label: "Memoria",
            children: settingsLoading ? (
              renderSettingsLoading()
            ) : (
              <Form
                form={settingsForm}
                layout="vertical"
                onFinish={handleSettingsSubmit}
              >
                <Form.Item
                  label="MEMORY_HALF_LIFE_DAYS"
                  name="MEMORY_HALF_LIFE_DAYS"
                >
                  <InputNumber min={1} max={3650} step={1} style={{ width: "100%" }} />
                </Form.Item>
                <Form.Item
                  label="SIMILARITY_THRESHOLD"
                  name="SIMILARITY_THRESHOLD"
                >
                  <InputNumber min={0} max={1} step={0.05} style={{ width: "100%" }} />
                </Form.Item>
                <Form.Item
                  label="RAG_BUDGET_TOKENS"
                  name="RAG_BUDGET_TOKENS"
                >
                  <InputNumber min={0} max={100000} step={100} style={{ width: "100%" }} />
                </Form.Item>
                <Form.Item
                  label="MEMORY_KNN_CANDIDATES"
                  name="MEMORY_KNN_CANDIDATES"
                >
                  <InputNumber min={1} max={1000} step={1} style={{ width: "100%" }} />
                </Form.Item>
                <Button type="primary" htmlType="submit" loading={saving}>
                  Guardar
                </Button>
              </Form>
            ),
          },
          {
            key: "persistent-memory",
            label: "Memoria persistente",
            children: settingsLoading ? (
              renderSettingsLoading()
            ) : (
              <PersistentMemoryPanel
                settings={settings}
                updateSettings={updateSettings}
                savingSettings={saving}
              />
            ),
          },
        ]}
      />
    </Modal>
  );
};