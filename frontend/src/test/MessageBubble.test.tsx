import { describe, it, expect } from "vitest";
import { render } from "@testing-library/react";
import { MessageBubble } from "../components/MessageBubble";
import type { Message } from "../types";

describe("MessageBubble", () => {
  it("renders user message right-aligned", () => {
    const msg: Message = {
      id: "1",
      role: "user",
      content: "Hola",
      created_at: "2024-01-01T00:00:00Z",
    };
    const { container } = render(<MessageBubble message={msg} />);
    const bubble = container.firstChild as HTMLElement;
    expect(bubble.style.justifyContent).toBe("flex-end");
  });

  it("renders assistant message left-aligned", () => {
    const msg: Message = {
      id: "2",
      role: "assistant",
      content: "Hola, soy Alfred",
      created_at: "2024-01-01T00:00:00Z",
    };
    const { container } = render(<MessageBubble message={msg} />);
    const bubble = container.firstChild as HTMLElement;
    expect(bubble.style.justifyContent).toBe("flex-start");
  });

  it("renders system message centered", () => {
    const msg: Message = {
      id: "3",
      role: "system",
      content: "System message",
      created_at: "2024-01-01T00:00:00Z",
    };
    const { container } = render(<MessageBubble message={msg} />);
    const bubble = container.firstChild as HTMLElement;
    expect(bubble.style.justifyContent).toBe("center");
  });

  it("renders tool role with monospace style", () => {
    const msg: Message = {
      id: "4",
      role: "tool",
      content: "Tool output",
      created_at: "2024-01-01T00:00:00Z",
    };
    const { container } = render(<MessageBubble message={msg} />);
    const bubble = container.firstChild as HTMLElement;
    expect(bubble.style.fontFamily).toBe("monospace");
  });

  // ════════════════════════════════════════════════════════════════
  // RED phase tests — expected to FAIL until react-markdown is added
  // ════════════════════════════════════════════════════════════════

  it("renders assistant markdown bold without literal asterisks", () => {
    const msg: Message = {
      id: "markdown-bold",
      role: "assistant",
      content: "Hello **world**",
      created_at: "2024-01-01T00:00:00Z",
    };
    const { container } = render(<MessageBubble message={msg} />);
    // BUG: Currently renders plain text, so "**world**" literal appears in output.
    // FIX: After react-markdown, bold text should not contain literal asterisks.
    expect(container.textContent).not.toContain("**world**");
  });

  it("renders assistant markdown code block as formatted code", () => {
    const msg: Message = {
      id: "markdown-code",
      role: "assistant",
      content: "```rust\nfn main() {}\n```",
      created_at: "2024-01-01T00:00:00Z",
    };
    const { container } = render(<MessageBubble message={msg} />);
    // BUG: Currently renders backticks literally. Fix should render formatted code block.
    expect(container.textContent).not.toContain("```rust");
  });

  it("renders user message as plain text (no markdown processing)", () => {
    const msg: Message = {
      id: "user-plain",
      role: "user",
      content: "Hello **world**",
      created_at: "2024-01-01T00:00:00Z",
    };
    const { container } = render(<MessageBubble message={msg} />);
    // User messages should remain plain text even if they contain markdown characters.
    expect(container.textContent).toContain("**world**");
  });

  // ════════════════════════════════════════════════════════════════
  // RED phase tests — expected to FAIL until timestamp/location added
  // ════════════════════════════════════════════════════════════════

  it("muestra timestamp formateado (HH:mm) para mensaje normal", () => {
    const msg: Message = {
      id: "5",
      role: "user",
      content: "Hola",
      created_at: "2026-09-27T10:30:00Z",
    };
    const { container } = render(<MessageBubble message={msg} />);
    // Debe contener un timestamp con formato HH:mm (two digits : two digits)
    expect(container.textContent).toMatch(/\d{2}:\d{2}/);
  });

  it("muestra ubicación cuando existe", () => {
    const msg: Message = {
      id: "6",
      role: "user",
      content: "Hola",
      location: "Silla, Valencia, España",
      created_at: "2026-09-27T10:30:00Z",
    };
    const { container } = render(<MessageBubble message={msg} />);
    expect(container.textContent).toContain("📍");
  });

  it("NO muestra ubicación cuando es null", () => {
    const msg: Message = {
      id: "7",
      role: "user",
      content: "Hola",
      location: null,
      created_at: "2026-09-27T10:30:00Z",
    };
    const { container } = render(<MessageBubble message={msg} />);
    expect(container.textContent).not.toContain("📍");
  });

  it("muestra la dirección completa, no truncada", () => {
    const msg: Message = {
      id: "8",
      role: "user",
      content: "Hola",
      location: "Calle Mayor 1, Silla, Valencia, España",
      created_at: "2026-09-27T10:30:00Z",
    };
    const { container } = render(<MessageBubble message={msg} />);
    expect(container.textContent).toContain("Calle Mayor 1");
    expect(container.textContent).toContain("Silla");
    expect(container.textContent).toContain("Valencia");
  });

  it("NO muestra timestamp para mensajes streaming", () => {
    const msg: Message = {
      id: "streaming",
      role: "assistant",
      content: "Escribiendo...",
      created_at: new Date().toISOString(),
    };
    const { container } = render(<MessageBubble message={msg} />);
    expect(container.textContent).not.toContain(":");
  });
});
