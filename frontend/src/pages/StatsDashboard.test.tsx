import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { StatsDashboard } from "./StatsDashboard";
import { api } from "../api/client";

const mockSummary = {
  total_calls: 150,
  total_prompt_tokens: 50000,
  total_completion_tokens: 30000,
  total_tokens: 80000,
  total_cached_tokens: 10000,
  total_reasoning_tokens: 5000,
  total_cost: 0.123456,
  total_errors: 3,
  avg_duration_ms: 2500,
};

const mockByModel = [
  { model: "gpt-4", calls: 100, total_tokens: 60000, total_cost: 0.1, avg_duration_ms: 3000, total_cached_tokens: 5000, total_reasoning_tokens: 2000 },
  { model: "gpt-3.5", calls: 50, total_tokens: 20000, total_cost: 0.023456, avg_duration_ms: 1500, total_cached_tokens: 5000, total_reasoning_tokens: 3000 },
];

const mockByDay = [
  { date: "2024-01-01", calls: 10, total_tokens: 5000, total_cost: 0.01, total_cached_tokens: 500, total_reasoning_tokens: 200 },
  { date: "2024-01-02", calls: 15, total_tokens: 7000, total_cost: 0.015, total_cached_tokens: 700, total_reasoning_tokens: 300 },
];

const mockTools = [
  { tool: "search", count: 45 },
  { tool: "calendar", count: 30 },
];

const mockDbSizes = [
  { table: "messages", rows: 5000 },
  { table: "users", rows: 100 },
];

vi.mock("../api/client", () => ({
  api: {
    getStatsSummary: vi.fn(),
    getStatsByModel: vi.fn(),
    getStatsByDay: vi.fn(),
    getStatsTools: vi.fn(),
    getDbSizes: vi.fn(),
    getRetention: vi.fn(),
    setRetention: vi.fn(),
  },
}));

describe("StatsDashboard", () => {
  beforeEach(() => {
    vi.clearAllMocks();
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

  it("renders loading state initially", () => {
    vi.mocked(api.getStatsSummary).mockReturnValue(new Promise(() => {}));
    vi.mocked(api.getStatsByModel).mockReturnValue(new Promise(() => {}));
    vi.mocked(api.getStatsByDay).mockReturnValue(new Promise(() => {}));
    vi.mocked(api.getStatsTools).mockReturnValue(new Promise(() => {}));
    vi.mocked(api.getDbSizes).mockReturnValue(new Promise(() => {}));

    const { container } = render(<StatsDashboard />);
    const spin = container.querySelector(".ant-spin-spinning");
    expect(spin).toBeInTheDocument();
  });

  it("renders components with data", async () => {
    vi.mocked(api.getStatsSummary).mockResolvedValue(mockSummary);
    vi.mocked(api.getStatsByModel).mockResolvedValue(mockByModel);
    vi.mocked(api.getStatsByDay).mockResolvedValue(mockByDay);
    vi.mocked(api.getStatsTools).mockResolvedValue(mockTools);
    vi.mocked(api.getDbSizes).mockResolvedValue(mockDbSizes);
    vi.mocked(api.getRetention).mockResolvedValue({ days: 30 });

    render(<StatsDashboard />);

    // Wait for data to load
    expect(await screen.findByText("LLM Usage Summary")).toBeInTheDocument();
    expect(await screen.findByText("Cost by Model")).toBeInTheDocument();
    expect(await screen.findByText("Daily Activity")).toBeInTheDocument();
    expect(await screen.findByText("Tool Usage")).toBeInTheDocument();
    expect(await screen.findByText("Database Sizes")).toBeInTheDocument();
    expect(await screen.findByText("Data Retention")).toBeInTheDocument();
  });

  it("renders empty state when no data", async () => {
    const emptySummary = {
      total_calls: 0,
      total_prompt_tokens: 0,
      total_completion_tokens: 0,
      total_tokens: 0,
      total_cached_tokens: 0,
      total_reasoning_tokens: 0,
      total_cost: 0,
      total_errors: 0,
      avg_duration_ms: null,
    };

    vi.mocked(api.getStatsSummary).mockResolvedValue(emptySummary);
    vi.mocked(api.getStatsByModel).mockResolvedValue([]);
    vi.mocked(api.getStatsByDay).mockResolvedValue([]);
    vi.mocked(api.getStatsTools).mockResolvedValue([]);
    vi.mocked(api.getDbSizes).mockResolvedValue([]);

    render(<StatsDashboard />);

    expect(await screen.findByText("No stats data available yet")).toBeInTheDocument();
  });

  it("renders error state", async () => {
    vi.mocked(api.getStatsSummary).mockRejectedValue(new Error("API error"));
    vi.mocked(api.getStatsByModel).mockRejectedValue(new Error("API error"));
    vi.mocked(api.getStatsByDay).mockRejectedValue(new Error("API error"));
    vi.mocked(api.getStatsTools).mockRejectedValue(new Error("API error"));
    vi.mocked(api.getDbSizes).mockRejectedValue(new Error("API error"));

    render(<StatsDashboard />);

    expect(await screen.findByText("Error loading stats")).toBeInTheDocument();
    expect(await screen.findByText("API error")).toBeInTheDocument();
  });
});