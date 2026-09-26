import React, { useState } from "react";
import { Drawer, Form, Input, Button, message } from "antd";
import type { Profile, UpdateProfile } from "../types";

interface ProfileEditorProps {
  profile: Profile | null;
  onUpdate: (data: UpdateProfile) => Promise<void>;
  visible: boolean;
  onClose: () => void;
}

export const ProfileEditor: React.FC<ProfileEditorProps> = ({
  profile,
  onUpdate,
  visible,
  onClose,
}) => {
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (values: UpdateProfile) => {
    setLoading(true);
    try {
      await onUpdate(values);
      message.success("Perfil actualizado");
      onClose();
    } catch {
      message.error("Error al actualizar perfil");
    } finally {
      setLoading(false);
    }
  };

  return (
    <Drawer title="Editar Perfil" open={visible} onClose={onClose} width={400}>
      <Form
        layout="vertical"
        initialValues={{
          name: profile?.name || "",
          avatar_url: profile?.avatar_url || "",
        }}
        onFinish={handleSubmit}
      >
        <Form.Item label="Nombre" name="name">
          <Input />
        </Form.Item>
        <Form.Item label="Avatar URL" name="avatar_url">
          <Input />
        </Form.Item>
        <Button type="primary" htmlType="submit" loading={loading}>
          Guardar
        </Button>
      </Form>
    </Drawer>
  );
};
