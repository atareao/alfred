import { describe, it, expect } from "vitest";
import { render, fireEvent } from "@testing-library/react";
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
      content: "Hola, soy Valet",
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

// ════════════════════════════════════════════════════════════════
// RED phase tests — chat-avatars (logo de Valet + avatar de usuario)
// ════════════════════════════════════════════════════════════════

function makeMessage(
  partial: Partial<Message> & Pick<Message, "id" | "role" | "content">,
): Message {
  return {
    created_at: "2026-09-27T10:30:00Z",
    ...partial,
  };
}

describe("MessageBubble — avatar del asistente (logo de Valet)", () => {
  it("muestra el logo de Valet y NO el icono robot", () => {
    const msg = makeMessage({
      id: "assistant-logo",
      role: "assistant",
      content: "Hola, soy Valet",
    });
    const { container } = render(<MessageBubble message={msg} />);

    const logo = container.querySelector("img[src*='valet-icon']");
    expect(logo).not.toBeNull();
    expect(logo!.getAttribute("alt")).toBeTruthy();
    expect(container.querySelector(".anticon-robot")).toBeNull();
  });

  it("el mensaje de streaming usa el mismo avatar de Valet y NO el icono robot", () => {
    const msg = makeMessage({
      id: "streaming",
      role: "assistant",
      content: "Escribiendo...",
    });
    const { container } = render(<MessageBubble message={msg} />);

    const logo = container.querySelector("img[src*='valet-icon']");
    expect(logo).not.toBeNull();
    expect(logo!.getAttribute("alt")).toBeTruthy();
    expect(container.querySelector(".anticon-robot")).toBeNull();
  });
});

describe("MessageBubble — roles system y tool conservan su icono", () => {
  it("system mantiene InfoCircleOutlined y no usa el logo de Valet", () => {
    const msg = makeMessage({ id: "sys", role: "system", content: "System message" });
    const { container } = render(<MessageBubble message={msg} />);

    expect(container.querySelector(".anticon-info-circle")).not.toBeNull();
    expect(container.querySelector("img[src*='valet-icon']")).toBeNull();
  });

  it("tool mantiene CodeOutlined y no usa el logo de Valet", () => {
    const msg = makeMessage({ id: "tool", role: "tool", content: "Tool output" });
    const { container } = render(<MessageBubble message={msg} />);

    expect(container.querySelector(".anticon-code")).not.toBeNull();
    expect(container.querySelector("img[src*='valet-icon']")).toBeNull();
  });
});

describe("MessageBubble — avatar del usuario", () => {
  const avatarUrl = "https://example.com/me.png";

  it("usuario con avatar configurado muestra la imagen y NO UserOutlined", () => {
    const msg = makeMessage({ id: "u-avatar", role: "user", content: "Hola" });
    const { container } = render(
      <MessageBubble message={msg} userAvatarUrl={avatarUrl} />,
    );

    const img = container.querySelector(`img[src="${avatarUrl}"]`);
    expect(img).not.toBeNull();
    expect(img!.getAttribute("alt")).toBeTruthy();
    expect(container.querySelector(".anticon-user")).toBeNull();
  });

  it("usuario sin avatar (null) degrada a UserOutlined", () => {
    const msg = makeMessage({ id: "u-null", role: "user", content: "Hola" });
    const { container } = render(
      <MessageBubble message={msg} userAvatarUrl={null} />,
    );

    expect(container.querySelector(".anticon-user")).not.toBeNull();
    expect(container.querySelector(`img[src="${avatarUrl}"]`)).toBeNull();
  });

  it("usuario sin avatar (cadena vacía) degrada a UserOutlined", () => {
    const msg = makeMessage({ id: "u-empty", role: "user", content: "Hola" });
    const { container } = render(
      <MessageBubble message={msg} userAvatarUrl="" />,
    );

    expect(container.querySelector(".anticon-user")).not.toBeNull();
    expect(container.querySelector(`img[src="${avatarUrl}"]`)).toBeNull();
  });

  it("el avatar del usuario NO se usa en roles assistant/system/tool", () => {
    for (const role of ["assistant", "system", "tool"] as const) {
      const msg = makeMessage({
        id: `other-${role}`,
        role,
        content: "contenido",
      });
      const { container, unmount } = render(
        <MessageBubble message={msg} userAvatarUrl={avatarUrl} />,
      );
      expect(container.querySelector(`img[src="${avatarUrl}"]`)).toBeNull();
      unmount();
    }
  });

  it("URL rota degrada a UserOutlined", () => {
    const brokenUrl = "https://example.com/roto.png";
    const msg = makeMessage({ id: "u-roto", role: "user", content: "Hola" });
    const { container } = render(
      <MessageBubble message={msg} userAvatarUrl={brokenUrl} />,
    );

    const img = container.querySelector(`img[src="${brokenUrl}"]`);
    expect(img).not.toBeNull();

    fireEvent.error(img!);

    expect(container.querySelector(".anticon-user")).not.toBeNull();
    expect(container.querySelector(`img[src="${brokenUrl}"]`)).toBeNull();
  });

  it("una URL válida no degrada", () => {
    const msg = makeMessage({ id: "u-valida", role: "user", content: "Hola" });
    const { container } = render(
      <MessageBubble message={msg} userAvatarUrl={avatarUrl} />,
    );

    expect(container.querySelector(`img[src="${avatarUrl}"]`)).not.toBeNull();
    expect(container.querySelector(".anticon-user")).toBeNull();
  });
});
