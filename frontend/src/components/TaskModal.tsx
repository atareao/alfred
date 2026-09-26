import React, { useEffect } from "react";
import { Modal, Form, Input, Select, DatePicker, message } from "antd";
import dayjs from "dayjs";
import type { Task } from "../types";
import { useCreateTask, useUpdateTask } from "../hooks/useTasks";

const { TextArea } = Input;

const STATUS_OPTIONS = [
  { value: "inbox", label: "📥 Inbox" },
  { value: "todo", label: "📋 Todo" },
  { value: "doing", label: "🏗️ Doing" },
  { value: "waiting", label: "⏳ Waiting" },
  { value: "someday", label: "📆 Someday" },
  { value: "done", label: "✅ Done" },
];

const PRIORITY_OPTIONS = [
  { value: "low", label: "Low" },
  { value: "medium", label: "Medium" },
  { value: "high", label: "High" },
];

const SCOPE_OPTIONS = [
  { value: "shared", label: "Shared" },
  { value: "personal", label: "Personal" },
];

interface TaskModalProps {
  open: boolean;
  task: Task | null;
  defaultStatus?: Task["status"];
  onClose: () => void;
  onSaved: () => void;
}

export const TaskModal: React.FC<TaskModalProps> = ({
  open,
  task,
  defaultStatus,
  onClose,
  onSaved,
}) => {
  const [form] = Form.useForm();
  const [messageApi, contextHolder] = message.useMessage();
  const { create, loading: creating } = useCreateTask();
  const { update, loading: updating } = useUpdateTask();
  const isEditing = !!task;

  useEffect(() => {
    if (open) {
      if (task) {
        form.setFieldsValue({
          content: task.content,
          status: task.status,
          priority: task.priority,
          project: task.project || "",
          due_date: task.due_date ? dayjs(task.due_date) : null,
          scope: task.scope,
        });
      } else {
        form.resetFields();
        form.setFieldsValue({
          status: defaultStatus || "inbox",
          priority: "medium",
          scope: "personal",
        });
      }
    }
  }, [open, task, defaultStatus, form]);

  const handleSubmit = async () => {
    try {
      const values = await form.validateFields();
      const data: Partial<Task> = {
        content: values.content,
        status: values.status || "inbox",
        priority: values.priority || "medium",
        project: values.project || undefined,
        due_date: values.due_date ? values.due_date.toISOString() : undefined,
        scope: values.scope || "personal",
      };

      if (isEditing && task) {
        await update(task.id, data);
        messageApi.success("Task updated");
      } else {
        await create(data);
        messageApi.success("Task created");
      }
      onSaved();
    } catch (err) {
      if (err && typeof err === "object" && "errorFields" in err) {
        return;
      }
      const msg = err instanceof Error ? err.message : "Failed to save task";
      messageApi.error(msg);
    }
  };

  return (
    <>
      {contextHolder}
      <Modal
        title={isEditing ? "Edit Task" : "New Task"}
        open={open}
        onOk={handleSubmit}
        onCancel={onClose}
        confirmLoading={creating || updating}
        destroyOnClose
        width={520}
      >
        <Form
          form={form}
          layout="vertical"
          initialValues={{
            status: "inbox",
            priority: "medium",
            scope: "personal",
          }}
        >
          <Form.Item
            name="content"
            label="Content"
            rules={[{ required: true, message: "Please enter task content" }]}
          >
            <TextArea rows={3} placeholder="What needs to be done?" />
          </Form.Item>

          <Form.Item name="status" label="Status">
            <Select options={STATUS_OPTIONS} />
          </Form.Item>

          <Form.Item name="priority" label="Priority">
            <Select options={PRIORITY_OPTIONS} />
          </Form.Item>

          <Form.Item name="project" label="Project">
            <Input placeholder="Project name (optional)" />
          </Form.Item>

          <Form.Item name="due_date" label="Due Date">
            <DatePicker showTime style={{ width: "100%" }} />
          </Form.Item>

          <Form.Item name="scope" label="Scope">
            <Select options={SCOPE_OPTIONS} />
          </Form.Item>
        </Form>
      </Modal>
    </>
  );
};

export default TaskModal;
