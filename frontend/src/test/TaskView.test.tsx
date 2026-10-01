import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import type { Task } from "../types";

// ---------------------------------------------------------------------------
// Ant Design matchMedia mock (jsdom does not implement window.matchMedia) —
// TaskView usa useMediaQuery.
// ---------------------------------------------------------------------------
beforeEach(() => {
  Object.defineProperty(window, "matchMedia", {
    writable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  });
});

const { mockUseTasks } = vi.hoisted(() => ({
  mockUseTasks: vi.fn(),
}));

vi.mock("../hooks/useTasks", () => ({
  useTasks: mockUseTasks,
  useCreateTask: vi.fn(() => ({ create: vi.fn(), loading: false })),
  useUpdateTask: vi.fn(() => ({ update: vi.fn(), loading: false })),
  useDeleteTask: vi.fn(() => ({ delete: vi.fn(), loading: false })),
}));

import { TaskView } from "../components/TaskView";
import { STATUS_COLORS } from "../components/task.constants";

const tasks: Task[] = [
  {
    id: "1",
    profile_id: "p1",
    content: "Buy milk",
    status: "todo",
    priority: "medium",
    scope: "shared",
    created_at: "2026-09-26T00:00:00Z",
    updated_at: "2026-09-26T00:00:00Z",
  },
];

describe("TaskView — caracterización", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockUseTasks.mockReturnValue({
      tasks,
      loading: false,
      error: null,
      refetch: vi.fn(),
    });
  });

  it("renderiza las columnas Kanban con su etiqueta y color de estado", () => {
    render(<TaskView />);

    const inbox = screen.getByText(/Inbox/);
    const todo = screen.getByText(/Todo/);
    const doing = screen.getByText(/Doing/);
    const done = screen.getByText(/Done/);

    expect(inbox).toHaveStyle({ color: STATUS_COLORS.inbox });
    expect(todo).toHaveStyle({ color: STATUS_COLORS.todo });
    expect(doing).toHaveStyle({ color: STATUS_COLORS.doing });
    expect(done).toHaveStyle({ color: STATUS_COLORS.done });
  });

  it("muestra el contenido de las tareas", () => {
    render(<TaskView />);

    expect(screen.getByText("Buy milk")).toBeInTheDocument();
  });
});
