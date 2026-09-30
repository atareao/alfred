import { describe, it, expect } from "vitest";
import { render, fireEvent } from "@testing-library/react";
// RED phase: este módulo todavía NO existe. El fichero de test debe fallar al
// importar hasta que la fase GREEN cree `frontend/src/components/UserAvatar.tsx`.
import { UserAvatar } from "../components/UserAvatar";

describe("UserAvatar", () => {
  it("sin src no renderiza ningún img y muestra UserOutlined", () => {
    const { container } = render(<UserAvatar src={null} />);

    expect(container.querySelector(".anticon-user")).not.toBeNull();
    expect(container.querySelector("img")).toBeNull();
  });

  it("con src válido renderiza img con alt, referrerpolicy y loading", () => {
    const url = "https://example.com/me.png";
    const { container } = render(<UserAvatar src={url} />);

    const img = container.querySelector("img");
    expect(img).not.toBeNull();
    expect(img!.getAttribute("src")).toBe(url);
    expect(img!.getAttribute("alt")).toBeTruthy();
    expect(img!.getAttribute("referrerpolicy")).toBe("no-referrer");
    expect(img!.getAttribute("loading")).toBe("lazy");
  });

  it("degrada a UserOutlined cuando la imagen dispara error", () => {
    const url = "https://example.com/me.png";
    const { container } = render(<UserAvatar src={url} />);

    const img = container.querySelector("img");
    expect(img).not.toBeNull();

    fireEvent.error(img!);

    expect(container.querySelector(".anticon-user")).not.toBeNull();
    expect(container.querySelector("img")).toBeNull();
  });

  it("recupera la imagen cuando cambia src tras degradar por error", () => {
    const broken = "https://example.com/roto.png";
    const fresh = "https://example.com/nuevo.png";
    const { container, rerender } = render(<UserAvatar src={broken} />);

    fireEvent.error(container.querySelector("img")!);
    expect(container.querySelector(".anticon-user")).not.toBeNull();

    rerender(<UserAvatar src={fresh} />);

    const img = container.querySelector("img");
    expect(img).not.toBeNull();
    expect(img!.getAttribute("src")).toBe(fresh);
    expect(container.querySelector(".anticon-user")).toBeNull();
  });

  it("trata un src en blanco como ausente", () => {
    const { container } = render(<UserAvatar src="   " />);

    expect(container.querySelector(".anticon-user")).not.toBeNull();
    expect(container.querySelector("img")).toBeNull();
  });
});
