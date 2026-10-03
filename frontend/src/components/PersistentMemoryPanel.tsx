import { useState } from "react";
import {
  Alert,
  App as AntdApp,
  Button,
  Divider,
  Input,
  InputNumber,
  Popconfirm,
  Space,
  Spin,
  Typography,
} from "antd";
import { usePersistentMemory } from "../hooks/usePersistentMemory";

const { Text } = Typography;
const { TextArea } = Input;

export interface PersistentMemoryPanelProps {
  settings: Record<string, string> | null;
  updateSettings: (data: Record<string, string>) => Promise<void>;
  savingSettings: boolean;
}

export function PersistentMemoryPanel({
  settings,
  updateSettings,
  savingSettings,
}: PersistentMemoryPanelProps) {
  const { message: messageApi } = AntdApp.useApp();
  const { state, loading, saving, error, warning, conflict, save, clear } =
    usePersistentMemory();

  const [draft, setDraft] = useState<string | null>(null);
  const [editBudget, setEditBudget] = useState<number | undefined>(undefined);

  const formatted = state?.payload ? JSON.stringify(state.payload, null, 2) : "";
  const text = draft ?? formatted;

  const settingsBudget = settings?.PERSISTENT_MEMORY_BUDGET_TOKENS;
  const budget =
    settingsBudget !== undefined && settingsBudget !== ""
      ? parseInt(settingsBudget, 10)
      : (state?.budget_tokens ?? 0);
  const budgetDraft = editBudget ?? (Number.isNaN(budget) ? null : budget);
  const tokenCount = state?.token_count ?? 0;
  const overBudget = tokenCount > budget;

  const handleSave = async () => {
    const outcome = await save(text);
    if (outcome === "saved" || outcome === "conflict") {
      setDraft(null);
    }
    if (outcome === "saved") {
      messageApi.success("Memoria persistente guardada");
    }
  };

  const handleClear = async () => {
    const ok = await clear();
    if (ok) {
      messageApi.success("Memoria persistente vaciada");
    }
  };

  const handleSaveBudget = async () => {
    if (budgetDraft === null) return;
    try {
      await updateSettings({
        PERSISTENT_MEMORY_BUDGET_TOKENS: budgetDraft.toString(),
      });
      setEditBudget(undefined);
      messageApi.success("Presupuesto guardado");
    } catch {
      messageApi.error("Error al guardar el presupuesto");
    }
  };

  if (loading) {
    return (
      <Spin
        style={{ display: "flex", justifyContent: "center", margin: "24px 0" }}
      />
    );
  }

  return (
    <div>
      <Space direction="vertical" style={{ width: "100%" }} size="middle">
        {error && <Alert type="error" message={error} showIcon />}
        {warning && <Alert type="warning" message={warning} showIcon />}
        {conflict && <Alert type="info" message={conflict} showIcon />}

        {state?.is_empty && (
          <Text type="secondary">No hay estado persistente guardado.</Text>
        )}

        <Text>Última actualización: {state?.updated_at ?? "—"}</Text>

        <Text type={overBudget ? "danger" : undefined}>
          {tokenCount} / {budget} tokens
          {overBudget ? " · Supera el presupuesto" : ""}
        </Text>

        <TextArea
          aria-label="Estado persistente"
          rows={12}
          value={text}
          onChange={(e) => setDraft(e.target.value)}
          spellCheck={false}
        />

        <Space>
          <Button type="primary" onClick={handleSave} loading={saving}>
            Guardar
          </Button>
          <Popconfirm
            title="¿Vaciar la memoria persistente?"
            description="Esta acción elimina el estado guardado y no se puede deshacer."
            okText="Sí"
            cancelText="No"
            onConfirm={handleClear}
          >
            <Button danger loading={saving}>
              Vaciar
            </Button>
          </Popconfirm>
        </Space>

        <Divider style={{ margin: "8px 0" }} />

        <Space align="center">
          <Text>PERSISTENT_MEMORY_BUDGET_TOKENS</Text>
          <InputNumber
            aria-label="PERSISTENT_MEMORY_BUDGET_TOKENS"
            min={1}
            max={1000000}
            step={100}
            value={budgetDraft}
            onChange={(value) => setEditBudget(value ?? undefined)}
          />
          <Button onClick={handleSaveBudget} loading={savingSettings}>
            Guardar presupuesto
          </Button>
        </Space>
      </Space>
    </div>
  );
}
