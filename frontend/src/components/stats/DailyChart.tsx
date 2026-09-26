import React from "react";
import { Card, Button, Space, Skeleton } from "antd";
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend,
  Filler,
} from "chart.js";
import { Line } from "react-chartjs-2";
import type { DayStats } from "../../types";

ChartJS.register(
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend,
  Filler,
);

interface DailyChartProps {
  data: DayStats[];
  loading: boolean;
  onRangeChange: (days: number) => void;
  selectedDays: number;
}

export const DailyChart: React.FC<DailyChartProps> = ({
  data,
  loading,
  onRangeChange,
  selectedDays,
}) => {
  if (loading) {
    return (
      <Card title="Daily Activity" style={{ marginBottom: 16 }}>
        <Skeleton active paragraph={{ rows: 6 }} />
      </Card>
    );
  }

  if (!data || data.length === 0) {
    return (
      <Card
        title="Daily Activity"
        extra={
          <RangeSelector selected={selectedDays} onChange={onRangeChange} />
        }
        style={{ marginBottom: 16 }}
      >
        <div style={{ textAlign: "center", padding: "24px 0", color: "rgba(255,255,255,0.45)" }}>
          No data yet
        </div>
      </Card>
    );
  }

  const sorted = [...data].sort(
    (a, b) => new Date(a.date).getTime() - new Date(b.date).getTime(),
  );

  const labels = sorted.map((d) => {
    const dt = new Date(d.date);
    return dt.toLocaleDateString(undefined, { month: "short", day: "numeric" });
  });

  const chartData = {
    labels,
    datasets: [
      {
        label: "Calls",
        data: sorted.map((d) => d.calls),
        borderColor: "rgba(22, 119, 255, 1)",
        backgroundColor: "rgba(22, 119, 255, 0.1)",
        fill: true,
        yAxisID: "y",
        tension: 0.3,
      },
      {
        label: "Cost ($)",
        data: sorted.map((d) => d.total_cost),
        borderColor: "rgba(255, 165, 0, 1)",
        backgroundColor: "rgba(255, 165, 0, 0.1)",
        fill: true,
        yAxisID: "y1",
        tension: 0.3,
      },
    ],
  };

  const options = {
    responsive: true,
    interaction: {
      mode: "index" as const,
      intersect: false,
    },
    plugins: {
      legend: {
        labels: { color: "rgba(255,255,255,0.65)" },
      },
    },
    scales: {
      x: {
        ticks: { color: "rgba(255,255,255,0.65)" },
        grid: { color: "rgba(255,255,255,0.1)" },
      },
      y: {
        type: "linear" as const,
        display: true,
        position: "left" as const,
        ticks: { color: "rgba(22, 119, 255, 0.8)" },
        grid: { color: "rgba(255,255,255,0.1)" },
        title: {
          display: true,
          text: "Calls",
          color: "rgba(22, 119, 255, 0.8)",
        },
      },
      y1: {
        type: "linear" as const,
        display: true,
        position: "right" as const,
        ticks: { color: "rgba(255, 165, 0, 0.8)" },
        grid: { drawOnChartArea: false },
        title: {
          display: true,
          text: "Cost ($)",
          color: "rgba(255, 165, 0, 0.8)",
        },
      },
    },
  };

  return (
    <Card
      title="Daily Activity"
      extra={
        <RangeSelector selected={selectedDays} onChange={onRangeChange} />
      }
      style={{ marginBottom: 16 }}
    >
      <Line data={chartData} options={options} />
    </Card>
  );
};

interface RangeSelectorProps {
  selected: number;
  onChange: (days: number) => void;
}

const RangeSelector: React.FC<RangeSelectorProps> = ({ selected, onChange }) => (
  <Space>
    <Button
      size="small"
      type={selected === 7 ? "primary" : "default"}
      onClick={() => onChange(7)}
    >
      7d
    </Button>
    <Button
      size="small"
      type={selected === 30 ? "primary" : "default"}
      onClick={() => onChange(30)}
    >
      30d
    </Button>
  </Space>
);