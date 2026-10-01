import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";
import { useTasks } from "../hooks/useTasks";
import type { Task } from "../types";
import { api } from "../api/client";

const mockTasks: Task[] = [
  {
    id: "1",
    profile_id: "p1",
    content: "Test task",
    status: "todo",
    priority: "medium",
    scope: "shared",
    created_at: "2026-09-26T00:00:00Z",
    updated_at: "2026-09-26T00:00:00Z",
  },
];

const mockNewTask: Task = {
  id: "2",
  profile_id: "p1",
  content: "New task",
  status: "done",
  priority: "low",
  scope: "shared",
  created_at: "2026-09-26T00:00:00Z",
  updated_at: "2026-09-26T00:00:00Z",
};

vi.mock("../api/client", () => ({
  api: {
    listTasks: vi.fn(),
  },
  BASE_URL: "http://localhost:3000",
}));

describe("useTasks", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(api.listTasks).mockResolvedValue(mockTasks);
  });

  afterEach(() => {
    // Clean up any custom event listeners between tests
    window.dispatchEvent(new CustomEvent("cleanup-test"));
  });

  it("fetches tasks on mount", async () => {
    const { result } = renderHook(() => useTasks());

    expect(result.current.loading).toBe(true);

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(api.listTasks).toHaveBeenCalledTimes(1);
    expect(api.listTasks).toHaveBeenCalledWith(undefined);
    expect(result.current.tasks).toEqual(mockTasks);
    expect(result.current.error).toBeNull();
  });

  it("passes filters to the API", async () => {
    renderHook(() => useTasks({ status: "todo" }));

    await waitFor(() => {
      expect(api.listTasks).toHaveBeenCalledTimes(1);
    });

    expect(api.listTasks).toHaveBeenCalledWith({ status: "todo" });
  });

  it("relaunches the request when refetch is called", async () => {
    const { result } = renderHook(() => useTasks());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(api.listTasks).toHaveBeenCalledTimes(1);

    vi.mocked(api.listTasks).mockResolvedValue([...mockTasks, mockNewTask]);

    await act(async () => {
      result.current.refetch();
    });

    await waitFor(() => {
      expect(api.listTasks).toHaveBeenCalledTimes(2);
    });

    expect(result.current.tasks).toHaveLength(2);
    expect(result.current.tasks[1].content).toBe("New task");
  });

  it("refetches when tasks-changed custom event is dispatched", async () => {
    const { result } = renderHook(() => useTasks());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(api.listTasks).toHaveBeenCalledTimes(1);

    vi.mocked(api.listTasks).mockResolvedValue([...mockTasks, mockNewTask]);

    window.dispatchEvent(new Event("tasks-changed"));

    await waitFor(() => {
      expect(api.listTasks).toHaveBeenCalledTimes(2);
    });

    expect(result.current.tasks).toHaveLength(2);
  });

  it("populates error when the request rejects", async () => {
    vi.mocked(api.listTasks).mockRejectedValue(new Error("boom"));

    const { result } = renderHook(() => useTasks());

    await waitFor(() => {
      expect(result.current.error).toBe("boom");
    });

    expect(result.current.loading).toBe(false);
  });
});
