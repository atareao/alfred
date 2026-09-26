import React, { useState, useEffect } from "react";
import { Card, InputNumber, Button, message, Skeleton } from "antd";
import { SaveOutlined } from "@ant-design/icons";
import { api } from "../../api/client";

export const RetentionConfig: React.FC = () => {
  const [days, setDays] = useState<number>(30);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    api
      .getRetention()
      .then((config) => {
        setDays(config.days);
        setLoading(false);
      })
      .catch(() => {
        message.error("Failed to load retention config");
        setLoading(false);
      });
  }, []);

  const handleSave = async () => {
    setSaving(true);
    try {
      await api.setRetention(days);
      message.success("Retention config updated");
    } catch {
      message.error("Failed to save retention config");
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <Card title="Data Retention" style={{ marginBottom: 16 }}>
        <Skeleton active paragraph={{ rows: 2 }} />
      </Card>
    );
  }

  return (
    <Card title="Data Retention" style={{ marginBottom: 16 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
        <span style={{ color: "rgba(255,255,255,0.65)" }}>
          Keep data for
        </span>
        <InputNumber
          min={7}
          max={365}
          value={days}
          onChange={(val) => val != null && setDays(val)}
          style={{ width: 80 }}
        />
        <span style={{ color: "rgba(255,255,255,0.65)" }}>days</span>
        <Button
          type="primary"
          icon={<SaveOutlined />}
          onClick={handleSave}
          loading={saving}
        >
          Save
        </Button>
      </div>
    </Card>
  );
};