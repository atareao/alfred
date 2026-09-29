import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { useSettings } from "../hooks/useSettings";

vi.mock("../api/client", () => ({
  api: {
    getSettings: vi.fn(),
    updateSettings: vi.fn(),
  },
}));

import { api } from "../api/client";

const mockGetSettings = vi.mocked(api.getSettings);
const mockUpdateSettings = vi.mocked(api.updateSettings);

describe("useSettings", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockGetSettings.mockResolvedValue({});
    mockUpdateSettings.mockResolvedValue({ max_window_tokens: "10000" });
  });

  it("resetToDefaults only sends max_window_tokens and never system_prompt", async () => {
    const { result } = renderHook(() => useSettings());

    await waitFor(() => {
      expect(mockGetSettings).toHaveBeenCalled();
    });

    await act(async () => {
      await result.current.resetToDefaults();
    });

    expect(mockUpdateSettings).toHaveBeenCalledTimes(1);
    const payload = mockUpdateSettings.mock.calls[0][0];
    expect(payload).toEqual({ max_window_tokens: "10000" });
    expect(payload).not.toHaveProperty("system_prompt");
  });
});
