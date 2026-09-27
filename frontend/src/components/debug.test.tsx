import { describe, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
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

describe("debug", () => {
  it("checks before opening", () => {
    render(<MemoryRouter><AppLayout /></MemoryRouter>);
    console.log("Before open:", screen.queryByTestId("stats-dashboard"));
    console.log("Close btns:", screen.queryAllByRole("button", { name: /close/i }).length);
    console.log("Close labels:", screen.queryAllByLabelText("Close").length);
  });
  it("checks after opening", () => {
    render(<MemoryRouter><AppLayout /></MemoryRouter>);
    const barChart = screen.getByRole("button", { name: /bar-chart/i });
    barChart.click();
    console.log("Has stats-dashboard:", screen.queryByTestId("stats-dashboard") !== null);
    console.log("Close btns:", screen.queryAllByRole("button", { name: /close/i }).length);
    // Print all close buttons
    const closeBtns = screen.queryAllByRole("button", { name: /close/i });
    closeBtns.forEach((b, i) => console.log(`close ${i}:`, b.outerHTML.substring(0, 150)));
  });
});
