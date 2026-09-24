import React, { useState, useEffect } from 'react';
import { Drawer, Form, InputNumber, Input, Button, message, Space } from 'antd';
import { useSettings } from '../hooks/useSettings';

const { TextArea } = Input;

interface SettingsEditorProps {
  visible: boolean;
  onClose: () => void;
}

export const SettingsEditor: React.FC<SettingsEditorProps> = ({ visible, onClose }) => {
  const { settings, loading, saving, updateSettings, resetToDefaults } = useSettings();
  const [form] = Form.useForm();
  const [resetting, setResetting] = useState(false);

  useEffect(() => {
    if (settings && visible) {
      form.setFieldsValue({
        max_window_tokens: parseInt(settings.max_window_tokens || '10000'),
        system_prompt: settings.system_prompt || '',
        message_page_size: parseInt(settings.message_page_size || '50'),
      });
    }
  }, [settings, visible, form]);

  const handleSubmit = async (values: { max_window_tokens: number; system_prompt: string; message_page_size: number }) => {
    try {
      await updateSettings({
        max_window_tokens: values.max_window_tokens.toString(),
        system_prompt: values.system_prompt,
        message_page_size: values.message_page_size.toString(),
      });
      message.success('Ajustes guardados');
      onClose();
    } catch {
      message.error('Error al guardar ajustes');
    }
  };

  const handleReset = async () => {
    setResetting(true);
    try {
      await resetToDefaults();
      form.setFieldsValue({
        max_window_tokens: 10000,
        system_prompt: '',
        message_page_size: 50,
      });
      message.success('Valores por defecto restaurados');
    } catch {
      message.error('Error al restaurar valores');
    } finally {
      setResetting(false);
    }
  };

  return (
    <Drawer
      title="Ajustes"
      open={visible}
      onClose={onClose}
      width={500}
      loading={loading}
    >
      <Form
        form={form}
        layout="vertical"
        onFinish={handleSubmit}
      >
        <Form.Item
          label="Ventana de contexto (tokens)"
          name="max_window_tokens"
          help="Máximo de tokens del historial que se envía al modelo. Modelo actual: 1M de contexto."
        >
          <InputNumber
            min={1000}
            max={100000}
            step={1000}
            style={{ width: '100%' }}
          />
        </Form.Item>

        <Form.Item
          label="System Prompt"
          name="system_prompt"
          help="Instrucciones personalizadas para Alfred. Vacío = usar prompt por defecto."
        >
          <TextArea rows={10} placeholder={settings?.system_prompt_default || "Dejar vacío para usar el prompt por defecto"} />
        </Form.Item>

        <Form.Item
          label="Tamaño de página"
          name="message_page_size"
          help="Número de mensajes por página al cargar el historial"
        >
          <InputNumber min={10} max={100} step={10} style={{ width: '100%' }} />
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
    </Drawer>
  );
};