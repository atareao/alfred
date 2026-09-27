import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { SummaryCard } from "./SummaryCard";
import type { StatsSummary } from "../../types";

const mockSummary: StatsSummary = {
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

describe("SummaryCard", () => {
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

  it("renders metrics when data is provided", () => {
    render(<SummaryCard data={mockSummary} loading={false} />);
    expect(screen.getByText("Total Calls")).toBeInTheDocument();
    expect(screen.getByText("150")).toBeInTheDocument();
    expect(screen.getByText("Total Tokens")).toBeInTheDocument();
    expect(screen.getByText("80,000")).toBeInTheDocument();
    expect(screen.getByText("Total Cost")).toBeInTheDocument();
    expect(screen.getByText(".123456")).toBeInTheDocument();
    expect(screen.getByText("Error Rate")).toBeInTheDocument();
    expect(screen.getByText("2")).toBeInTheDocument();
    expect(screen.getByText(".00")).toBeInTheDocument();
    expect(screen.getByText("2500 ms")).toBeInTheDocument();
  });

  it("renders skeleton when loading", () => {
    const { container } = render(
      <SummaryCard data={null} loading={true} />,
    );
    const skeleton = container.querySelector(".ant-skeleton");
    expect(skeleton).toBeInTheDocument();
  });

  it("renders 'No data yet' when data is null", () => {
    render(<SummaryCard data={null} loading={false} />);
    expect(screen.getByText("No data yet")).toBeInTheDocument();
  });

  it("renders 'No data yet' when total_calls is 0", () => {
    const emptySummary: StatsSummary = {
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
    render(<SummaryCard data={emptySummary} loading={false} />);
    expect(screen.getByText("No data yet")).toBeInTheDocument();
  });
});