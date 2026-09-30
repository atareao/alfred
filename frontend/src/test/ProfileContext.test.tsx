import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { Profile } from "../types";

// ---------------------------------------------------------------------------
// Mock del cliente API (mismo patrón que useSettings.test.ts)
// ---------------------------------------------------------------------------
vi.mock("../api/client", () => ({
  api: {
    getProfile: vi.fn(),
    updateProfile: vi.fn(),
  },
}));

import { api } from "../api/client";
import { ProfileProvider } from "../contexts/ProfileProvider";
import { useProfileContext } from "../contexts/ProfileContext";

const mockGetProfile = vi.mocked(api.getProfile);
const mockUpdateProfile = vi.mocked(api.updateProfile);

const baseProfile: Profile = {
  id: "p1",
  name: "Lorenzo",
  avatar_url: "https://example.com/viejo.png",
  preferences: {},
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
};

describe("ProfileContext", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("useProfileContext fuera del provider lanza un error explícito", () => {
    const consoleError = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});

    function Consumer() {
      const { profile } = useProfileContext();
      return <div>{profile?.name ?? "sin perfil"}</div>;
    }

    expect(() => render(<Consumer />)).toThrow();

    consoleError.mockRestore();
  });

  it("un único provider comparte el perfil y llama a getProfile una sola vez", async () => {
    mockGetProfile.mockResolvedValue(baseProfile);

    function Consumer({ label }: { label: string }) {
      const { profile } = useProfileContext();
      return <div data-testid={label}>{profile?.name ?? ""}</div>;
    }

    render(
      <ProfileProvider>
        <Consumer label="consumer-a" />
        <Consumer label="consumer-b" />
      </ProfileProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId("consumer-a")).toHaveTextContent("Lorenzo");
      expect(screen.getByTestId("consumer-b")).toHaveTextContent("Lorenzo");
    });

    expect(mockGetProfile).toHaveBeenCalledTimes(1);
  });

  it("editar el avatar actualiza el contexto sin recargar", async () => {
    const user = userEvent.setup();
    mockGetProfile.mockResolvedValue(baseProfile);
    mockUpdateProfile.mockResolvedValue({
      ...baseProfile,
      avatar_url: "https://example.com/nuevo.png",
    });

    function Probe() {
      const { profile, updateProfile } = useProfileContext();
      return (
        <div>
          <span data-testid="avatar">{profile?.avatar_url ?? ""}</span>
          <button
            onClick={() =>
              void updateProfile({
                avatar_url: "https://example.com/nuevo.png",
              })
            }
          >
            guardar
          </button>
        </div>
      );
    }

    render(
      <ProfileProvider>
        <Probe />
      </ProfileProvider>,
    );

    await waitFor(() =>
      expect(screen.getByTestId("avatar")).toHaveTextContent("viejo.png"),
    );

    await user.click(screen.getByRole("button", { name: "guardar" }));

    await waitFor(() =>
      expect(screen.getByTestId("avatar")).toHaveTextContent("nuevo.png"),
    );
  });
});
