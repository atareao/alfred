import { Card, Descriptions, Tag, Spin, Empty } from "antd";
import type { LastApiCall } from "../../types";

interface Props {
  data: LastApiCall | null;
  loading: boolean;
}

function formatJson(jsonStr: string | null): string {
  if (!jsonStr) return "—";
  try {
    return JSON.stringify(JSON.parse(jsonStr), null, 2);
  } catch {
    return jsonStr;
  }
}

function statusColor(status: string): string {
  switch (status) {
    case "success":
      return "green";
    case "error":
      return "red";
    case "timeout":
      return "orange";
    default:
      return "default";
  }
}

export const LastApiCallCard: React.FC<Props> = ({ data, loading }) => {
  if (loading) {
    return (
      <Card title="📡 Última llamada a OpenRouter">
        <div style={{ textAlign: "center", padding: 24 }}>
          <Spin />
        </div>
      </Card>
    );
  }

  if (!data) {
    return (
      <Card title="📡 Última llamada a OpenRouter">
        <Empty description="No hay llamadas registradas" />
      </Card>
    );
  }

  return (
    <Card title="📡 Última llamada a OpenRouter">
      <Descriptions column={2} size="small" bordered>
        <Descriptions.Item label="Modelo">{data.model}</Descriptions.Item>
        <Descriptions.Item label="Estado">
          <Tag color={statusColor(data.status)}>{data.status}</Tag>
        </Descriptions.Item>
        <Descriptions.Item label="Tokens (prompt)">
          {data.prompt_tokens.toLocaleString()}
        </Descriptions.Item>
        <Descriptions.Item label="Tokens (completion)">
          {data.completion_tokens.toLocaleString()}
        </Descriptions.Item>
        <Descriptions.Item label="Tokens (total)">
          {data.total_tokens.toLocaleString()}
        </Descriptions.Item>
        <Descriptions.Item label="Tokens (cached)">
          {data.cached_tokens.toLocaleString()}
        </Descriptions.Item>
        <Descriptions.Item label="Tokens (reasoning)">
          {data.reasoning_tokens.toLocaleString()}
        </Descriptions.Item>
        <Descriptions.Item label="Coste">
          ${data.cost.toFixed(6)}
        </Descriptions.Item>
        <Descriptions.Item label="Duración">
          {data.duration_ms ? `${data.duration_ms} ms` : "—"}
        </Descriptions.Item>
        <Descriptions.Item label="Fecha">{data.created_at}</Descriptions.Item>
        {data.error_message && (
          <Descriptions.Item label="Error" span={2}>
            <span style={{ color: "red" }}>{data.error_message}</span>
          </Descriptions.Item>
        )}
      </Descriptions>

      <div style={{ marginTop: 16 }}>
        <h4>Request Body</h4>
        <pre
          style={{
            background: "#1e1e1e",
            color: "#d4d4d4",
            padding: 12,
            borderRadius: 6,
            fontSize: 12,
            overflowX: "auto",
            maxHeight: 300,
            whiteSpace: "pre-wrap",
            wordBreak: "break-all",
          }}
        >
          {formatJson(data.request_body)}
        </pre>
      </div>

      <div style={{ marginTop: 16 }}>
        <h4>Response Body</h4>
        <pre
          style={{
            background: "#1e1e1e",
            color: "#d4d4d4",
            padding: 12,
            borderRadius: 6,
            fontSize: 12,
            overflowX: "auto",
            maxHeight: 300,
            whiteSpace: "pre-wrap",
            wordBreak: "break-all",
          }}
        >
          {formatJson(data.response_body)}
        </pre>
      </div>
    </Card>
  );
};