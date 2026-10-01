import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App as AntdApp } from "antd";

// ---------------------------------------------------------------------------
// Ant Design matchMedia mock (jsdom does not implement window.matchMedia).
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

// `vi.hoisted` ensures the mocks exist before the hoisted `vi.mock` factory runs.
const { mockCreate, mockUpdate } = vi.hoisted(() => ({
  mockCreate: vi.fn(),
  mockUpdate: vi.fn(),
}));

vi.mock("../hooks/useTasks", () => ({
  useCreateTask: vi.fn(() => ({ create: mockCreate, loading: false })),
  useUpdateTask: vi.fn(() => ({ update: mockUpdate, loading: false })),
}));

import { TaskModal } from "../components/TaskModal";

describe("TaskModal", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("usa el messageApi contextual en el camino de error", async () => {
    const user = userEvent.setup();
    mockCreate.mockRejectedValue(new Error("boom"));

    render(
      <AntdApp>
        <TaskModal
          open
          task={null}
          defaultStatus="inbox"
          onClose={vi.fn()}
          onSaved={vi.fn()}
        />
      </AntdApp>,
    );

    await user.type(screen.getByLabelText("Content"), "Una tarea");
    await user.click(screen.getByRole("button", { name: "OK" }));

    // El aviso de error sale por el contexto de antd.
    expect(await screen.findByText("boom")).toBeInTheDocument();
  });

  it("avisa el éxito por el contexto al crear una tarea", async () => {
    const user = userEvent.setup();
    const onSaved = vi.fn();
    mockCreate.mockResolvedValue({});

    render(
      <AntdApp>
        <TaskModal
          open
          task={null}
          defaultStatus="inbox"
          onClose={vi.fn()}
          onSaved={onSaved}
        />
      </AntdApp>,
    );

    await user.type(screen.getByLabelText("Content"), "Una tarea");
    await user.click(screen.getByRole("button", { name: "OK" }));

    expect(await screen.findByText("Task created")).toBeInTheDocument();
    expect(onSaved).toHaveBeenCalled();
  });
});
