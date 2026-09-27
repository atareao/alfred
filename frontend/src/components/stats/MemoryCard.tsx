import React from "react";
import { Card, Row, Col, Statistic, Skeleton } from "antd";
import type { MemoryStats } from "../../types";

interface MemoryCardProps {
  data: MemoryStats | null;
  loading: boolean;
}

export const MemoryCard: React.FC<MemoryCardProps> = ({ data, loading }) => {
  if (loading) {
    return (
      <Card>
        <Skeleton active paragraph={{ rows: 4 }} />
      </Card>
    );
  }

  if (!data || data.total_memories === 0) {
    return (
      <Card>
        <div style={{ textAlign: "center", padding: "24px 0", color: "rgba(255,255,255,0.45)" }}>
          No data yet
        </div>
      </Card>
    );
  }

  const indexedRatio =
    data.messages_total > 0
      ? ((data.messages_indexed / data.messages_total) * 100).toFixed(1)
      : "0.0";

  return (
    <Card title="🧠 Memory" style={{ marginBottom: 16 }}>
      <Row gutter={[16, 16]}>
        <Col xs={12} sm={8} md={6}>
          <Statistic title="Total Memories" value={data.total_memories} />
        </Col>
        <Col xs={12} sm={8} md={6}>
          <Statistic title="Total Tokens Stored" value={data.total_tokens} />
        </Col>
        <Col xs={24} sm={16} md={12}>
          <Statistic
            title="Indexed Messages Ratio"
            value={`${data.messages_indexed} / ${data.messages_total} = ${indexedRatio}%`}
          />
        </Col>
      </Row>
    </Card>
  );
};