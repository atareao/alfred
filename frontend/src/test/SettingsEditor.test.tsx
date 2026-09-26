import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { SettingsEditor } from "../components/SettingsEditor";

// ---------------------------------------------------------------------------
// Ant Design uses window.matchMedia which is not available in jsdom
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

// ---------------------------------------------------------------------------
// Mock useSettings hook
// ---------------------------------------------------------------------------
vi.mock("../hooks/useSettings", () => ({
  useSettings: vi.fn(() => ({
    settings: {
      max_window_tokens: "10000",
      system_prompt: "",
      message_page_size: "50",
    },
    loading: false,
    saving: false,
    error: null,
    updateSettings: vi.fn(),
    resetToDefaults: vi.fn(),
  })),
}));

describe("SettingsEditor", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // -----------------------------------------------------------------------
  // message_page_size field — Configurable page size
  //
  // Current SettingsEditor only has max_window_tokens and system_prompt.
  // After the change, it SHALL also include a message_page_size field
  // with min=10, max=100, step=10.
  // -----------------------------------------------------------------------
  it("renders message_page_size field with correct constraints", () => {
    render(<SettingsEditor visible={true} onClose={vi.fn()} />);

    // ------------------------------------------------------------------
    // RED phase assertion — this SHOULD FAIL because SettingsEditor
    // currently does not have a message_page_size field.
    //
    // After the GREEN phase, SettingsEditor will include an InputNumber
    // with name="message_page_size", min=10, max=100, step=10.
    // ------------------------------------------------------------------

    // Try to find the field by its label text
    const pageSizeInput = screen.queryByLabelText(
      /tamaño de página|message_page_size|page size/i,
    );
    expect(pageSizeInput).not.toBeNull();

    // Verify it's an input element with the right constraints
    if (pageSizeInput) {
      const input = pageSizeInput as HTMLInputElement;
      // Ant Design v5 InputNumber uses aria-valuemin/aria-valuemax instead of min/max
      expect(input).toHaveAttribute("aria-valuemin", "10");
      expect(input).toHaveAttribute("aria-valuemax", "100");
      expect(input).toHaveAttribute("step", "10");
    }
  });
});
