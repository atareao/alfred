import { useState, useEffect } from "react";
import { Row, Col, Spin, Alert, Empty } from "antd";
import { api } from "../api/client";
import type { StatsSummary, ModelStats, DayStats, ToolStats, TableSize } from "../types";
import { SummaryCard } from "../components/stats/SummaryCard";
import { ModelChart } from "../components/stats/ModelChart";
import { DailyChart } from "../components/stats/DailyChart";
import { ToolsChart } from "../components/stats/ToolsChart";
import { DbSizesTable } from "../components/stats/DbSizesTable";
import { RetentionConfig } from "../components/stats/RetentionConfig";

export const StatsDashboard: React.FC = () => {
  const [summary, setSummary] = useState<StatsSummary | null>(null);
  const [byModel, setByModel] = useState<ModelStats[]>([]);
  const [byDay, setByDay] = useState<DayStats[]>([]);
  const [tools, setTools] = useState<ToolStats[]>([]);
  const [dbSizes, setDbSizes] = useState<TableSize[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedDays, setSelectedDays] = useState(30);
  const [error, setError] = useState<string | null>(null);

  const loadData = async (days: number) => {
    setLoading(true);
    setError(null);
    try {
      const [summaryData, modelData, dayData, toolsData, dbData] =
        await Promise.all([
          api.getStatsSummary(),
          api.getStatsByModel(),
          api.getStatsByDay(days),
          api.getStatsTools(),
          api.getDbSizes(),
        ]);
      setSummary(summaryData);
      setByModel(modelData);
      setByDay(dayData);
      setTools(toolsData);
      setDbSizes(dbData);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load stats");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData(selectedDays);
  }, [selectedDays]);

  const handleRangeChange = (days: number) => {
    setSelectedDays(days);
  };

  if (error) {
    return (
      <div style={{ padding: 24 }}>
        <Alert
          message="Error loading stats"
          description={error}
          type="error"
          showIcon
          action={
            <a onClick={() => loadData(selectedDays)} style={{ cursor: "pointer" }}>
              Retry
            </a>
          }
        />
      </div>
    );
  }

  if (loading && !summary && !byModel.length && !byDay.length) {
    return (
      <div
        style={{
          display: "flex",
          justifyContent: "center",
          alignItems: "center",
          minHeight: "60vh",
        }}
      >
        <Spin size="large" tip="Loading stats..." />
      </div>
    );
  }

  const hasData =
    (summary && summary.total_calls > 0) ||
    byModel.length > 0 ||
    byDay.length > 0 ||
    tools.length > 0 ||
    dbSizes.length > 0;

  if (!loading && !hasData) {
    return (
      <div style={{ padding: 24 }}>
        <Empty description="No stats data available yet" />
      </div>
    );
  }

  return (
    <div style={{ padding: 24 }}>
      <Row gutter={[16, 16]}>
        <Col span={24}>
          <SummaryCard data={summary} loading={loading} />
        </Col>
        <Col xs={24} lg={12}>
          <ModelChart data={byModel} loading={loading} />
        </Col>
        <Col xs={24} lg={12}>
          <DailyChart
            data={byDay}
            loading={loading}
            onRangeChange={handleRangeChange}
            selectedDays={selectedDays}
          />
        </Col>
        <Col xs={24} lg={8}>
          <ToolsChart data={tools} loading={loading} />
        </Col>
        <Col xs={24} lg={8}>
          <DbSizesTable data={dbSizes} loading={loading} />
        </Col>
        <Col xs={24} lg={8}>
          <RetentionConfig />
        </Col>
      </Row>
    </div>
  );
};