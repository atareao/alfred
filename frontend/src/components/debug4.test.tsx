import { describe, it, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { AppLayout } from "./AppLayout";
import { ProfileProvider } from "../contexts/ProfileProvider";

vi.mock("../pages/StatsDashboard", () => ({
  StatsDashboard: () => <div data-testid="stats-dashboard">Stats Content</div>,
}));
vi.mock("./ChatView", () => ({ ChatView: () => <div data-testid="chat-view">Chat</div> }));
vi.mock("./ProfileEditor", () => ({ ProfileEditor: () => null }));
vi.mock("./SettingsEditor", () => ({ SettingsEditor: () => null }));
vi.mock("./CalendarView", () => ({ CalendarView: () => null }));
vi.mock("./TaskView", () => ({ TaskView: () => null }));
vi.mock("../hooks/useMainChat", () => ({
  useMainChat: () => ({ messages: [], loading: false, sendMessage: vi.fn(), streaming: false, streamingContent: null, activeTools: [] }),
}));
vi.mock("../hooks/useProfile", () => ({
  useProfile: () => ({ profile: { id: "test", name: "Test", preferences: "{}" }, loading: false, error: null, updateProfile: vi.fn() }),
}));
vi.mock("../hooks/useSettings", () => ({
  useSettings: () => ({ settings: {}, loading: false, saving: false, error: null, updateSettings: vi.fn(), resetToDefaults: vi.fn() }),
}));

describe("debug4", () => {
  it("try clicking various things", () => {
    render(<MemoryRouter><ProfileProvider><AppLayout /></ProfileProvider></MemoryRouter>);
    const barChart = screen.getByRole("button", { name: /bar-chart/i });
    fireEvent.click(barChart);
    console.log("After open - stats:", screen.queryByTestId("stats-dashboard") !== null);
    
    // Try clicking the X icon button
    const closeBtn = screen.getByRole("button", { name: /close/i });
    console.log("Button outerHTML:", closeBtn.outerHTML);
    
    // Try dispatchEvent
    const event = new MouseEvent("click", { bubbles: true, cancelable: true });
    closeBtn.dispatchEvent(event);
    console.log("After dispatch - stats:", screen.queryByTestId("stats-dashboard") !== null);
    
    // Check if there's a mask we can click
    const mask = document.querySelector(".ant-modal-mask");
    console.log("Mask found:", !!mask);
    if (mask) {
      const maskEvent = new MouseEvent("click", { bubbles: true, cancelable: true });
      mask.dispatchEvent(maskEvent);
      console.log("After mask click - stats:", screen.queryByTestId("stats-dashboard") !== null);
    }
    
    // What about mousedown on close?
    fireEvent.mouseDown(closeBtn);
    fireEvent.mouseUp(closeBtn);
    console.log("After mousedown/mouseup - stats:", screen.queryByTestId("stats-dashboard") !== null);
  });
});
