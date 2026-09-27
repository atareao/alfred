import React from "react";
import { Card, Table, Skeleton } from "antd";
import { Chart as ChartJS, ArcElement, Tooltip, Legend } from "chart.js";
import { Doughnut } from "react-chartjs-2";
import type { ToolStats } from "../../types";

ChartJS.register(ArcElement, Tooltip, Legend);

interface ToolsChartProps {
  data: ToolStats[];
  loading: boolean;
}

const COLORS = [
  "rgba(22, 119, 255, 0.8)",
  "rgba(255, 165, 0, 0.8)",
  "rgba(82, 196, 26, 0.8)",
  "rgba(255, 77, 79, 0.8)",
  "rgba(114, 46, 209, 0.8)",
  "rgba(0, 204, 150, 0.8)",
  "rgba(255, 193, 7, 0.8)",
  "rgba(23, 162, 184, 0.8)",
];

export const ToolsChart: React.FC<ToolsChartProps> = ({ data, loading }) => {
  if (loading) {
    return (
      <Card title="Tool Usage" style={{ marginBottom: 16 }}>
        <Skeleton active paragraph={{ rows: 6 }} />
      </Card>
    );
  }

  if (!data || data.length === 0) {
    return (
      <Card title="Tool Usage" style={{ marginBottom: 16 }}>
        <div style={{ textAlign: "center", padding: "24px 0", color: "rgba(255,255,255,0.45)" }}>
          No data yet
        </div>
      </Card>
    );
  }

  const chartData = {
    labels: data.map((t) => t.tool),
    datasets: [
      {
        data: data.map((t) => t.count),
        backgroundColor: COLORS.slice(0, data.length),
        borderColor: COLORS.slice(0, data.length).map((c) =>
          c.replace("0.8", "1"),
        ),
        borderWidth: 1,
      },
    ],
  };

  const options = {
    responsive: true,
    plugins: {
      legend: {
        position: "bottom" as const,
        labels: { color: "rgba(255,255,255,0.65)" },
      },
    },
  };

  const columns = [
    { title: "Tool", dataIndex: "tool", key: "tool" },
    { title: "Count", dataIndex: "count", key: "count" },
  ];

  return (
    <Card title="Tool Usage" style={{ marginBottom: 16 }}>
      <div style={{ maxWidth: 300, margin: "0 auto 16px" }}>
        <Doughnut data={chartData} options={options} />
      </div>
      <Table
        dataSource={data}
        columns={columns}
        rowKey="tool"
        pagination={false}
        size="small"
      />
    </Card>
  );
};