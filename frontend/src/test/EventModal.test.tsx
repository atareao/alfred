import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { message } from "antd";
import dayjs from "dayjs";

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

vi.mock("../hooks/useEvents", () => ({
  useCreateEvent: vi.fn(() => ({ create: mockCreate, loading: false })),
  useUpdateEvent: vi.fn(() => ({ update: mockUpdate, loading: false })),
}));

import { EventModal } from "../components/EventModal";

describe("EventModal", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("usa el messageApi contextual (y NO el message estático) en el camino de error", async () => {
    const user = userEvent.setup();
    // Espiamos sin neutralizar: queremos detectar si el antipatrón vuelve.
    const errorSpy = vi.spyOn(message, "error");
    mockCreate.mockRejectedValue(new Error("boom"));

    render(
      <EventModal
        open
        event={null}
        defaultDate={dayjs()}
        onClose={vi.fn()}
        onSaved={vi.fn()}
      />,
    );

    await user.type(screen.getByLabelText("Title"), "Reunión");
    await user.click(screen.getByRole("button", { name: "OK" }));

    // El aviso de error sale por el contextHolder del propio componente.
    await waitFor(() => {
      expect(screen.getByText("boom")).toBeInTheDocument();
    });

    // Aserción que caza la regresión al antipatrón.
    expect(errorSpy).not.toHaveBeenCalled();
  });
});
