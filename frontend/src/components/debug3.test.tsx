import { describe, it, vi } from "vitest";
import { render, screen, fireEvent, act } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { AppLayout } from "./AppLayout";

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

describe("debug3", () => {
  it("opens, closes, and checks", () => {
    render(<MemoryRouter><AppLayout /></MemoryRouter>);
    const barChart = screen.getByRole("button", { name: /bar-chart/i });
    fireEvent.click(barChart);
    console.log("After open - stats:", screen.queryByTestId("stats-dashboard") !== null);
    
    const closeBtn = screen.getByRole("button", { name: /close/i });
    console.log("Close button found:", closeBtn.className);
    
    act(() => { fireEvent.click(closeBtn); });
    console.log("After close - stats:", screen.queryByTestId("stats-dashboard") !== null);
    
    // Check the close button is gone
    console.log("Close btns after:", screen.queryAllByRole("button", { name: /close/i }).length);
  });
});
