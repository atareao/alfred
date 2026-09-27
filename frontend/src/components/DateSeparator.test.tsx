import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { DateSeparator } from "./DateSeparator";

describe("DateSeparator", () => {
  it('muestra "Hoy" para la fecha actual', () => {
    const today = new Date().toISOString();
    render(<DateSeparator date={today} />);
    expect(screen.getByText("Hoy")).toBeTruthy();
  });

  it('muestra "Ayer" para el día anterior', () => {
    const yesterday = new Date(Date.now() - 86400000).toISOString();
    render(<DateSeparator date={yesterday} />);
    expect(screen.getByText("Ayer")).toBeTruthy();
  });

  it("muestra fecha completa para días más antiguos", () => {
    render(<DateSeparator date="2026-09-25T10:00:00Z" />);
    expect(screen.getByText("25 sept 2026")).toBeTruthy();
  });

  it("tiene el estilo de línea divisoria", () => {
    const { container } = render(<DateSeparator date="2026-09-27T10:00:00Z" />);
    const element = container.firstChild as HTMLElement;
    expect(element.style.borderBottom).toBeTruthy();
  });
});