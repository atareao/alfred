import React, { useState, useMemo } from "react";
import {
  Button,
  Space,
  Typography,
  Spin,
  Alert,
  Segmented,
  Tag,
  Badge,
  Table,
  Tooltip,
  Popconfirm,
  Select,
} from "antd";
import {
  PlusOutlined,
  EditOutlined,
  DeleteOutlined,
  FilterOutlined,
} from "@ant-design/icons";
import dayjs from "dayjs";
import type { Task } from "../types";
import { useTasks, useUpdateTask, useDeleteTask } from "../hooks/useTasks";
import { TaskModal } from "./TaskModal";
import { useMediaQuery } from "../hooks/useMediaQuery";

const { Text, Title } = Typography;

export const STATUS_COLORS: Record<string, string> = {
  inbox: "#8c8c8c",
  todo: "#1677ff",
  doing: "#faad14",
  waiting: "#722ed1",
  someday: "#13c2c2",
  done: "#52c41a",
};

export const STATUS_LABELS: Record<string, string> = {
  inbox: "📥 Inbox",
  todo: "📋 Todo",
  doing: "🏗️ Doing",
  waiting: "⏳ Waiting",
  someday: "📆 Someday",
  done: "✅ Done",
};

const PRIORITY_COLORS: Record<string, string> = {
  low: "#8c8c8c",
  medium: "#faad14",
  high: "#f5222d",
};

const KANBAN_COLUMNS: Task["status"][] = ["inbox", "todo", "doing", "done"];

interface TaskViewProps {
  onClose?: () => void;
}

export const TaskView: React.FC<TaskViewProps> = ({ onClose: _onClose }) => {
  const [viewMode, setViewMode] = useState<"kanban" | "list">("kanban");
  const [modalOpen, setModalOpen] = useState(false);
  const [editingTask, setEditingTask] = useState<Task | null>(null);
  const [defaultStatus, setDefaultStatus] = useState<
    Task["status"] | undefined
  >(undefined);
  const [listFilterStatus, setListFilterStatus] = useState<
    Task["status"] | undefined
  >(undefined);
  const [listFilterPriority, setListFilterPriority] = useState<
    string | undefined
  >(undefined);
  const [listFilterProject, setListFilterProject] = useState<
    string | undefined
  >(undefined);

  // Build filters for the API call — only send non-empty filters
  const apiFilters = useMemo(() => {
    const f: Record<string, string> = {};
    if (listFilterStatus) f.status = listFilterStatus;
    if (listFilterPriority) f.priority = listFilterPriority;
    if (listFilterProject) f.project = listFilterProject;
    return Object.keys(f).length > 0 ? f : undefined;
  }, [listFilterStatus, listFilterPriority, listFilterProject]);

  const { tasks, loading, error, refetch } = useTasks(
    viewMode === "list" ? apiFilters : undefined,
  );
  const { update } = useUpdateTask();
  const { delete: deleteTask } = useDeleteTask();
  const isMobile = useMediaQuery("(max-width: 767px)");

  // Group tasks by status for Kanban
  const tasksByStatus = useMemo(() => {
    const map: Record<string, Task[]> = {};
    for (const s of KANBAN_COLUMNS) {
      map[s] = [];
    }
    // Also add waiting and someday if they exist
    map["waiting"] = [];
    map["someday"] = [];
    for (const t of tasks) {
      if (map[t.status]) {
        map[t.status].push(t);
      } else {
        // fallback for unknown status
        map[t.status] = [t];
      }
    }
    return map;
  }, [tasks]);

  // Quick filter counts for list view
  const waitingCount = useMemo(
    () => tasks.filter((t) => t.status === "waiting").length,
    [tasks],
  );
  const somedayCount = useMemo(
    () => tasks.filter((t) => t.status === "someday").length,
    [tasks],
  );
  // Unique projects for filter
  const projects = useMemo(() => {
    const p = new Set<string>();
    tasks.forEach((t) => {
      if (t.project) p.add(t.project);
    });
    return Array.from(p).sort();
  }, [tasks]);

  const handleCreateTask = (status?: Task["status"]) => {
    setEditingTask(null);
    setDefaultStatus(status);
    setModalOpen(true);
  };

  const handleEditTask = (task: Task) => {
    setEditingTask(task);
    setModalOpen(true);
  };

  const handleModalClose = () => {
    setModalOpen(false);
    setEditingTask(null);
    setDefaultStatus(undefined);
  };

  const handleTaskSaved = () => {
    handleModalClose();
    refetch();
  };

  const handleStatusChange = async (task: Task, newStatus: Task["status"]) => {
    if (task.status === newStatus) return;
    try {
      await update(task.id, { status: newStatus });
      refetch();
    } catch {
      // Error handled by the hook
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await deleteTask(id);
      refetch();
    } catch {
      // Error handled by the hook
    }
  };

  const formatDate = (dateStr?: string) => {
    if (!dateStr) return null;
    const d = dayjs(dateStr);
    const now = dayjs();
    if (d.isSame(now, "day")) return `Today ${d.format("HH:mm")}`;
    if (d.isSame(now.add(1, "day"), "day"))
      return `Tomorrow ${d.format("HH:mm")}`;
    return d.format("MMM D");
  };

  // ── Kanban column renderer ──
  const renderKanbanColumn = (status: Task["status"]) => {
    const columnTasks = tasksByStatus[status] || [];
    return (
      <div
        key={status}
        style={{
          flex: 1,
          minWidth: isMobile ? "100%" : 220,
          maxWidth: isMobile ? "100%" : 300,
          background: "rgba(255,255,255,0.03)",
          borderRadius: 8,
          padding: 12,
          display: "flex",
          flexDirection: "column",
        }}
      >
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            marginBottom: 12,
          }}
        >
          <Text strong style={{ color: STATUS_COLORS[status], fontSize: 14 }}>
            {STATUS_LABELS[status]}{" "}
            <Badge
              count={columnTasks.length}
              style={{ backgroundColor: STATUS_COLORS[status], fontSize: 11 }}
            />
          </Text>
          <Button
            type="text"
            size="small"
            icon={<PlusOutlined />}
            onClick={() => handleCreateTask(status)}
            style={{ color: "rgba(255,255,255,0.45)" }}
          />
        </div>
        <div
          style={{ flex: 1, display: "flex", flexDirection: "column", gap: 8 }}
        >
          {columnTasks.map((task) => (
            <div
              key={task.id}
              draggable
              onDragStart={(e) => {
                e.dataTransfer.setData(
                  "text/plain",
                  JSON.stringify({ id: task.id, status: task.status }),
                );
              }}
              onDragOver={(e) => e.preventDefault()}
              onDrop={(e) => {
                e.preventDefault();
                try {
                  const data = JSON.parse(e.dataTransfer.getData("text/plain"));
                  if (data.id && data.status !== status) {
                    handleStatusChange(task, status);
                    // Re-fetch after drop
                    refetch();
                  }
                } catch {
                  /* ignore invalid drops */
                }
              }}
              style={{
                background: "#1f1f1f",
                borderRadius: 6,
                padding: "8px 10px",
                cursor: "grab",
                borderLeft: `3px solid ${STATUS_COLORS[task.status]}`,
                transition: "box-shadow 0.2s",
              }}
              onClick={() => handleEditTask(task)}
            >
              <Text
                style={{
                  color: "rgba(255,255,255,0.85)",
                  fontSize: 13,
                  display: "block",
                  marginBottom: 6,
                  wordBreak: "break-word",
                }}
              >
                {task.content}
              </Text>
              <Space size={4} wrap>
                <Tag
                  color={PRIORITY_COLORS[task.priority]}
                  style={{ fontSize: 11, lineHeight: "16px", margin: 0 }}
                >
                  {task.priority}
                </Tag>
                {task.project && (
                  <Tag
                    style={{
                      fontSize: 11,
                      lineHeight: "16px",
                      margin: 0,
                      background: "rgba(22,119,255,0.15)",
                      border: "none",
                      color: "#1677ff",
                    }}
                  >
                    {task.project}
                  </Tag>
                )}
                {task.due_date && (
                  <Text
                    style={{ fontSize: 11, color: "rgba(255,255,255,0.45)" }}
                  >
                    {formatDate(task.due_date)}
                  </Text>
                )}
              </Space>
            </div>
          ))}
          {columnTasks.length === 0 && (
            <div style={{ textAlign: "center", padding: "16px 0" }}>
              <Text style={{ color: "rgba(255,255,255,0.25)", fontSize: 12 }}>
                No tasks
              </Text>
            </div>
          )}
        </div>
      </div>
    );
  };

  // ── List view table columns ──
  const tableColumns = [
    {
      title: "Content",
      dataIndex: "content",
      key: "content",
      ellipsis: true,
      render: (text: string) => (
        <Text style={{ color: "rgba(255,255,255,0.85)" }}>{text}</Text>
      ),
    },
    {
      title: "Status",
      dataIndex: "status",
      key: "status",
      width: 120,
      render: (status: Task["status"]) => (
        <Tag color={STATUS_COLORS[status]} style={{ margin: 0 }}>
          {STATUS_LABELS[status]}
        </Tag>
      ),
    },
    {
      title: "Priority",
      dataIndex: "priority",
      key: "priority",
      width: 90,
      render: (priority: Task["priority"]) => (
        <Tag color={PRIORITY_COLORS[priority]} style={{ margin: 0 }}>
          {priority}
        </Tag>
      ),
    },
    {
      title: "Project",
      dataIndex: "project",
      key: "project",
      width: 130,
      ellipsis: true,
      render: (project?: string) =>
        project ? (
          <Tag
            style={{
              background: "rgba(22,119,255,0.15)",
              border: "none",
              color: "#1677ff",
              margin: 0,
            }}
          >
            {project}
          </Tag>
        ) : null,
    },
    {
      title: "Due",
      dataIndex: "due_date",
      key: "due_date",
      width: 100,
      render: (date?: string) =>
        date ? (
          <Text style={{ color: "rgba(255,255,255,0.45)", fontSize: 12 }}>
            {formatDate(date)}
          </Text>
        ) : null,
    },
    {
      title: "Actions",
      key: "actions",
      width: 80,
      render: (_: unknown, record: Task) => (
        <Space size={0}>
          <Tooltip title="Edit">
            <Button
              type="text"
              size="small"
              icon={<EditOutlined />}
              onClick={() => handleEditTask(record)}
              style={{ color: "rgba(255,255,255,0.45)" }}
            />
          </Tooltip>
          <Popconfirm
            title="Delete this task?"
            onConfirm={() => handleDelete(record.id)}
            okText="Delete"
            cancelText="Cancel"
          >
            <Tooltip title="Delete">
              <Button
                type="text"
                size="small"
                icon={<DeleteOutlined />}
                style={{ color: "rgba(255,255,255,0.45)" }}
              />
            </Tooltip>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  if (error) {
    return (
      <Alert type="error" message={error} showIcon style={{ margin: 16 }} />
    );
  }

  return (
    <div style={{ padding: 16, minHeight: 400 }}>
      <Space direction="vertical" style={{ width: "100%" }} size="middle">
        {/* Header row */}
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            flexWrap: "wrap",
            gap: 8,
          }}
        >
          <Title
            level={4}
            style={{ margin: 0, color: "rgba(255,255,255,0.85)" }}
          >
            ✅ Tasks
          </Title>
          <Space
            direction={isMobile ? "vertical" : "horizontal"}
            style={{ width: isMobile ? "100%" : undefined }}
          >
            <Segmented
              value={viewMode}
              onChange={(val) => setViewMode(val as "kanban" | "list")}
              options={[
                { value: "kanban", label: "Kanban" },
                { value: "list", label: "List" },
              ]}
            />
            <Button
              type="primary"
              icon={<PlusOutlined />}
              onClick={() => handleCreateTask()}
              block={isMobile}
            >
              New Task
            </Button>
          </Space>
        </div>

        {/* Loading state */}
        {loading && viewMode === "kanban" ? (
          <div style={{ textAlign: "center", padding: 40 }}>
            <Spin size="large" />
          </div>
        ) : null}

        {/* ── Kanban view ── */}
        {viewMode === "kanban" && !loading && (
          <div
            style={{
              display: "flex",
              gap: 12,
              overflowX: "auto",
              paddingBottom: 8,
              flexWrap: isMobile ? "wrap" : "nowrap",
            }}
          >
            {KANBAN_COLUMNS.map(renderKanbanColumn)}
          </div>
        )}

        {/* ── List view ── */}
        {viewMode === "list" && (
          <>
            {/* Quick filter badges */}
            <Space size={4} wrap>
              <Badge
                count={waitingCount}
                style={{ backgroundColor: STATUS_COLORS.waiting, fontSize: 11 }}
                offset={[4, -2]}
              >
                <Button
                  size="small"
                  type={listFilterStatus === "waiting" ? "primary" : "default"}
                  onClick={() =>
                    setListFilterStatus(
                      listFilterStatus === "waiting" ? undefined : "waiting",
                    )
                  }
                  style={{ fontSize: 12 }}
                >
                  ⏳ Waiting
                </Button>
              </Badge>
              <Badge
                count={somedayCount}
                style={{ backgroundColor: STATUS_COLORS.someday, fontSize: 11 }}
                offset={[4, -2]}
              >
                <Button
                  size="small"
                  type={listFilterStatus === "someday" ? "primary" : "default"}
                  onClick={() =>
                    setListFilterStatus(
                      listFilterStatus === "someday" ? undefined : "someday",
                    )
                  }
                  style={{ fontSize: 12 }}
                >
                  📆 Someday
                </Button>
              </Badge>
              {/* General filter dropdowns */}
              <Select
                allowClear
                placeholder="Status"
                style={{ width: 110 }}
                size="small"
                value={listFilterStatus}
                onChange={setListFilterStatus}
                options={Object.entries(STATUS_LABELS).map(
                  ([value, label]) => ({ value, label }),
                )}
              />
              <Select
                allowClear
                placeholder="Priority"
                style={{ width: 100 }}
                size="small"
                value={listFilterPriority}
                onChange={setListFilterPriority}
                options={[
                  { value: "low", label: "Low" },
                  { value: "medium", label: "Medium" },
                  { value: "high", label: "High" },
                ]}
              />
              <Select
                allowClear
                placeholder="Project"
                style={{ width: 140 }}
                size="small"
                value={listFilterProject}
                onChange={setListFilterProject}
                options={projects.map((p) => ({ value: p, label: p }))}
              />
              {(listFilterStatus ||
                listFilterPriority ||
                listFilterProject) && (
                <Button
                  size="small"
                  icon={<FilterOutlined />}
                  onClick={() => {
                    setListFilterStatus(undefined);
                    setListFilterPriority(undefined);
                    setListFilterProject(undefined);
                  }}
                >
                  Clear
                </Button>
              )}
            </Space>

            {/* Table */}
            {loading ? (
              <div style={{ textAlign: "center", padding: 40 }}>
                <Spin size="large" />
              </div>
            ) : (
              <Table
                dataSource={tasks}
                columns={tableColumns}
                rowKey="id"
                pagination={{
                  pageSize: 20,
                  showSizeChanger: false,
                  size: "small",
                }}
                size="small"
                locale={{
                  emptyText: (
                    <Text style={{ color: "rgba(255,255,255,0.45)" }}>
                      No tasks found
                    </Text>
                  ),
                }}
                style={{ background: "transparent" }}
                components={{
                  header: {
                    cell: (props: React.HTMLAttributes<HTMLElement>) => (
                      <th
                        {...props}
                        style={{
                          ...props.style,
                          background: "rgba(255,255,255,0.04)",
                          color: "rgba(255,255,255,0.65)",
                          borderBottom: "1px solid rgba(255,255,255,0.08)",
                        }}
                      />
                    ),
                  },
                  body: {
                    row: (props: React.HTMLAttributes<HTMLElement>) => (
                      <tr
                        {...props}
                        style={{
                          ...props.style,
                          borderBottom: "1px solid rgba(255,255,255,0.06)",
                        }}
                      />
                    ),
                    cell: (props: React.HTMLAttributes<HTMLElement>) => (
                      <td
                        {...props}
                        style={{ ...props.style, background: "transparent" }}
                      />
                    ),
                  },
                }}
              />
            )}
          </>
        )}
      </Space>

      <TaskModal
        open={modalOpen}
        task={editingTask}
        defaultStatus={defaultStatus}
        onClose={handleModalClose}
        onSaved={handleTaskSaved}
      />
    </div>
  );
};

export default TaskView;
