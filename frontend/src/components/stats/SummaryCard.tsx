import React from "react";
import { Card, Row, Col, Statistic, Skeleton } from "antd";
import type { StatsSummary } from "../../types";

interface SummaryCardProps {
  data: StatsSummary | null;
  loading: boolean;
}

export const SummaryCard: React.FC<SummaryCardProps> = ({ data, loading }) => {
  if (loading) {
    return (
      <Card>
        <Skeleton active paragraph={{ rows: 4 }} />
      </Card>
    );
  }

  if (!data || data.total_calls === 0) {
    return (
      <Card>
        <div style={{ textAlign: "center", padding: "24px 0", color: "rgba(255,255,255,0.45)" }}>
          No data yet
        </div>
      </Card>
    );
  }

  const errorRate =
    data.total_calls > 0
      ? ((data.total_errors / data.total_calls) * 100).toFixed(2)
      : "0.00";

  return (
    <Card title="LLM Usage Summary" style={{ marginBottom: 16 }}>
      <Row gutter={[16, 16]}>
        <Col xs={12} sm={8} md={6}>
          <Statistic title="Total Calls" value={data.total_calls} />
        </Col>
        <Col xs={12} sm={8} md={6}>
          <Statistic title="Total Tokens" value={data.total_tokens} />
        </Col>
        <Col xs={12} sm={8} md={6}>
          <Statistic
            title="Prompt Tokens"
            value={data.total_prompt_tokens}
          />
        </Col>
        <Col xs={12} sm={8} md={6}>
          <Statistic
            title="Completion Tokens"
            value={data.total_completion_tokens}
          />
        </Col>
        <Col xs={12} sm={8} md={6}>
          <Statistic title="Cached Tokens" value={data.total_cached_tokens} />
        </Col>
        <Col xs={12} sm={8} md={6}>
          <Statistic
            title="Reasoning Tokens"
            value={data.total_reasoning_tokens}
          />
        </Col>
        <Col xs={12} sm={8} md={6}>
          <Statistic
            title="Total Cost"
            value={data.total_cost.toFixed(6)}
            prefix="$"
          />
        </Col>
        <Col xs={12} sm={8} md={6}>
          <Statistic title="Error Rate" value={errorRate} suffix="%" />
        </Col>
        <Col xs={12} sm={8} md={6}>
          <Statistic
            title="Avg Duration"
            value={data.avg_duration_ms != null ? `${data.avg_duration_ms.toFixed(0)} ms` : "N/A"}
          />
        </Col>
        <Col xs={12} sm={8} md={6}>
          <Statistic title="Total Errors" value={data.total_errors} />
        </Col>
      </Row>
    </Card>
  );
};