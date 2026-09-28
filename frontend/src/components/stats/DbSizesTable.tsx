import React from "react";
import { Card, Table, Skeleton } from "antd";
import type { TableSize } from "../../types";

interface DbSizesTableProps {
  data: TableSize[];
  loading: boolean;
}

export const DbSizesTable: React.FC<DbSizesTableProps> = ({ data, loading }) => {
  if (loading) {
    return (
      <Card title="Database Sizes" style={{ marginBottom: 16 }}>
        <Skeleton active paragraph={{ rows: 6 }} />
      </Card>
    );
  }

  if (!data || data.length === 0) {
    return (
      <Card title="Database Sizes" style={{ marginBottom: 16 }}>
        <div style={{ textAlign: "center", padding: "24px 0", color: "rgba(255,255,255,0.45)" }}>
          No data yet
        </div>
      </Card>
    );
  }

  const sorted = [...data].sort((a, b) => b.rows - a.rows);

  const headerStyle = { color: "#ffffff" };
  const columns = [
    { title: "Table", dataIndex: "table", key: "table", onHeaderCell: () => ({ style: headerStyle }) },
    { title: "Rows", dataIndex: "rows", key: "rows", onHeaderCell: () => ({ style: headerStyle }) },
  ];

  return (
    <Card title="Database Sizes" style={{ marginBottom: 16 }}>
      <Table
        dataSource={sorted}
        columns={columns}
        rowKey="table"
        pagination={false}
        size="small"
      />
    </Card>
  );
};