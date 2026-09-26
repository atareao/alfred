import React from "react";
import { Modal, Descriptions, Tag, Button, Space, Popconfirm } from "antd";
import { DeleteOutlined, EditOutlined } from "@ant-design/icons";
import type { CalendarEvent } from "../types";
import { useMediaQuery } from "../hooks/useMediaQuery";

const CATEGORY_COLORS: Record<string, string> = {
  default: "#1677ff",
  work: "#52c41a",
  personal: "#fa8c16",
  health: "#f5222d",
  birthday: "#eb2f96",
  holiday: "#722ed1",
};

interface EventDetailProps {
  open: boolean;
  event: CalendarEvent | null;
  onClose: () => void;
  onEdit: (event: CalendarEvent) => void;
  onDelete: (id: string) => void;
}

export const EventDetail: React.FC<EventDetailProps> = ({
  open,
  event,
  onClose,
  onEdit,
  onDelete,
}) => {
  const isMobile = useMediaQuery("(max-width: 767px)");
  if (!event) return null;

  const formatDate = (iso: string) => {
    const d = new Date(iso);
    return d.toLocaleString("es-ES", {
      dateStyle: "long",
      timeStyle: "short",
    });
  };

  return (
    <Modal
      title={event.title}
      open={open}
      onCancel={onClose}
      footer={
        <Space>
          <Button icon={<EditOutlined />} onClick={() => onEdit(event)}>
            Edit
          </Button>
          <Popconfirm
            title="Delete event"
            description="Are you sure you want to delete this event?"
            onConfirm={() => onDelete(event.id)}
            okText="Delete"
            cancelText="Cancel"
            okButtonProps={{ danger: true }}
          >
            <Button danger icon={<DeleteOutlined />}>
              Delete
            </Button>
          </Popconfirm>
        </Space>
      }
      width={isMobile ? "calc(100vw - 32px)" : 480}
    >
      <Descriptions column={1} bordered={!isMobile} size="small">
        <Descriptions.Item label="Category">
          <Tag color={CATEGORY_COLORS[event.category] || "#1677ff"}>
            {event.category}
          </Tag>
        </Descriptions.Item>
        <Descriptions.Item label="Start">
          {formatDate(event.start_time)}
        </Descriptions.Item>
        <Descriptions.Item label="End">
          {formatDate(event.end_time)}
        </Descriptions.Item>
        {event.all_day && (
          <Descriptions.Item label="All Day">Yes</Descriptions.Item>
        )}
        {event.location && (
          <Descriptions.Item label="Location">
            {event.location}
          </Descriptions.Item>
        )}
        {event.description && (
          <Descriptions.Item label="Description">
            {event.description}
          </Descriptions.Item>
        )}
        {event.rrule && (
          <Descriptions.Item label="Recurrence">
            {event.rrule}
          </Descriptions.Item>
        )}
        {event.reminder_minutes_before != null && (
          <Descriptions.Item label="Reminder">
            {event.reminder_minutes_before} min before
          </Descriptions.Item>
        )}
        <Descriptions.Item label="Scope">{event.scope}</Descriptions.Item>
      </Descriptions>
    </Modal>
  );
};
