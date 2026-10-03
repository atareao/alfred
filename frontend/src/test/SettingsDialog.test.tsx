import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor, within, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App as AntdApp } from "antd";
import type { ReactElement, ReactNode } from "react";

// ---------------------------------------------------------------------------
// Ant Design matchMedia mock
// ---------------------------------------------------------------------------
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

const mockUpdateProfile = vi.fn();
const mockUpdateSettings = vi.fn();
const mockResetToDefaults = vi.fn();

// Fixture de settings devuelta por el hook mockeado (equivale a lo que
// responde `GET /settings`). Es mutable para que cada test pueda simular
// una respuesta distinta sin redefinir el módulo.
const defaultSettings: Record<string, string> = {
  max_window_tokens: "10000",
  system_prompt: "Eres Valet",
  archivist_prompt: "Eres un archivista",
  collapse_prompt: "Resume el texto",
  consolidator_prompt:
    "Consolida {{ ESTADO_ACTUAL }} con {{ BLOQUE_DE_MENSAJES }}",
  font_size: "16",
  message_page_size: "50",
  openweather_api_key: "",
  google_places_api_key: "",
  brave_search_api_key: "",
  MEMORY_HALF_LIFE_DAYS: "30",
  SIMILARITY_THRESHOLD: "0.4",
  RAG_BUDGET_TOKENS: "400",
  MEMORY_KNN_CANDIDATES: "10",
};

let mockSettings: Record<string, string> = { ...defaultSettings };

// ---------------------------------------------------------------------------
// Mock hooks
// ---------------------------------------------------------------------------
vi.mock("../hooks/useProfile", () => ({
  useProfile: vi.fn(() => ({
    profile: { id: "test", name: "Test User", avatar_url: "", preferences: "{}" },
    loading: false,
    error: null,
    updateProfile: mockUpdateProfile,
  })),
}));

vi.mock("../hooks/useSettings", () => ({
  useSettings: vi.fn(() => ({
    settings: mockSettings,
    loading: false,
    saving: false,
    error: null,
    updateSettings: mockUpdateSettings,
    resetToDefaults: mockResetToDefaults,
  })),
}));

import { SettingsDialog } from "../components/SettingsDialog";
import { ProfileProvider } from "../contexts/ProfileProvider";

// antd `App.useApp()` exige un `<App>` ancestro. Sin él el contexto por defecto
// son objetos vacíos: `messageApi.success` sería `undefined` y lanzaría un
// TypeError (fallo ruidoso, no un fallback silencioso a la API estática).
// Todo montaje va envuelto para ejercitar el camino contextual.
const AppWrapper = ({ children }: { children: ReactNode }) => (
  <AntdApp>{children}</AntdApp>
);

const renderDialog = (ui: ReactElement) => render(ui, { wrapper: AppWrapper });

describe("SettingsDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockSettings = { ...defaultSettings };
  });

  it("is not visible when visible=false", () => {
    renderDialog(<ProfileProvider><SettingsDialog visible={false} onClose={vi.fn()} /></ProfileProvider>);
    expect(screen.queryByText("⚙️ Settings")).not.toBeInTheDocument();
  });

  it("renders modal with correct title when visible", () => {
    renderDialog(<ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>);
    expect(screen.getByText("⚙️ Settings")).toBeInTheDocument();
  });

  it("renders Perfil tab by default with name and avatar fields", () => {
    renderDialog(<ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>);
    expect(screen.getByText("Perfil")).toBeInTheDocument();
    expect(screen.getByLabelText("Nombre")).toBeInTheDocument();
    expect(screen.getByLabelText("Avatar URL")).toBeInTheDocument();
  });

  it("saves profile when submitting Perfil tab", async () => {
    const user = userEvent.setup();
    mockUpdateProfile.mockResolvedValue(undefined);
    renderDialog(<ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>);

    const nameInput = screen.getByLabelText("Nombre");
    await user.clear(nameInput);
    await user.type(nameInput, "Juan");

    await user.click(screen.getByRole("button", { name: /guardar/i }));

    await waitFor(() => {
      expect(mockUpdateProfile).toHaveBeenCalledWith(
        expect.objectContaining({ name: "Juan" }),
      );
    });

    // REFACTOR (tarea 2.3): el aviso se asevera sobre el DOM — `<App>` lo
    // renderiza dentro del contenedor de RTL.
    expect(await screen.findByText("Perfil actualizado")).toBeInTheDocument();
  });

  it("shows error message when profile save fails", async () => {
    const user = userEvent.setup();
    mockUpdateProfile.mockRejectedValue(new Error("fail"));
    renderDialog(<ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>);

    await user.click(screen.getByRole("button", { name: /guardar/i }));

    await waitFor(() => {
      expect(screen.getByText("Error al actualizar perfil")).toBeInTheDocument();
    });
  });

  it("renders Interfaz tab with font size, context window, and page size", async () => {
    const user = userEvent.setup();
    renderDialog(<ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>);

    // Click on Interfaz tab
    await user.click(screen.getByText("Interfaz"));

    expect(screen.getByText("Tamaño de fuente")).toBeInTheDocument();
    expect(screen.getByText("Ventana de contexto (tokens)")).toBeInTheDocument();
    expect(screen.getByText("Tamaño de página")).toBeInTheDocument();
  });

  it("renders Prompts tab with System, Archivist, Collapse and Consolidator sub-tabs", async () => {
    const user = userEvent.setup();
    renderDialog(<ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>);

    await user.click(screen.getByText("Prompts"));

    expect(screen.getByText("System")).toBeInTheDocument();
    expect(screen.getByText("Archivist")).toBeInTheDocument();
    expect(screen.getByText("Collapse")).toBeInTheDocument();
    expect(screen.getByText("Consolidator")).toBeInTheDocument();
  });

  it("shows system_prompt when opening the System sub-tab", async () => {
    const user = userEvent.setup();
    renderDialog(<ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>);

    await user.click(screen.getByText("Prompts"));
    await user.click(screen.getByText("System"));

    expect(screen.getByLabelText("System Prompt")).toHaveValue("Eres Valet");
  });

  it("shows archivist_prompt when opening the Archivist sub-tab", async () => {
    const user = userEvent.setup();
    renderDialog(<ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>);

    await user.click(screen.getByText("Prompts"));
    await user.click(screen.getByText("Archivist"));

    expect(screen.getByLabelText("Archivist Prompt")).toHaveValue(
      "Eres un archivista",
    );
  });

  it("shows collapse_prompt when opening the Collapse sub-tab", async () => {
    const user = userEvent.setup();
    renderDialog(<ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>);

    await user.click(screen.getByText("Prompts"));
    await user.click(screen.getByText("Collapse"));

    expect(screen.getByLabelText("Collapse Prompt")).toHaveValue(
      "Resume el texto",
    );
  });

  it("saves the three prompts together via updateSettings", async () => {
    const user = userEvent.setup();
    mockUpdateSettings.mockResolvedValue(undefined);
    renderDialog(<ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>);

    await user.click(screen.getByText("Prompts"));

    const system = screen.getByLabelText("System Prompt");
    await user.clear(system);
    await user.type(system, "Nuevo system");

    const archivist = screen.getByLabelText("Archivist Prompt");
    await user.clear(archivist);
    await user.type(archivist, "Nuevo archivist");

    const collapse = screen.getByLabelText("Collapse Prompt");
    await user.clear(collapse);
    await user.type(collapse, "Nuevo collapse");

    const form = system.closest("form") as HTMLFormElement;
    await user.click(within(form).getByRole("button", { name: /guardar/i }));

    await waitFor(() => {
      expect(mockUpdateSettings).toHaveBeenCalledWith(
        expect.objectContaining({
          system_prompt: "Nuevo system",
          archivist_prompt: "Nuevo archivist",
          collapse_prompt: "Nuevo collapse",
        }),
      );
    });

    expect(await screen.findByText("Ajustes guardados")).toBeInTheDocument();
  });

  it("keeps the loaded prompts when saving from the Interfaz tab", async () => {
    const user = userEvent.setup();
    mockUpdateSettings.mockResolvedValue(undefined);
    renderDialog(<ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>);

    await user.click(screen.getByText("Interfaz"));

    const fontSize = screen.getByLabelText("Tamaño de fuente");
    await user.clear(fontSize);
    await user.type(fontSize, "20");

    const form = fontSize.closest("form") as HTMLFormElement;
    await user.click(within(form).getByRole("button", { name: /guardar/i }));

    await waitFor(() => {
      expect(mockUpdateSettings).toHaveBeenCalledWith(
        expect.objectContaining({
          font_size: "20",
          system_prompt: "Eres Valet",
          archivist_prompt: "Eres un archivista",
          collapse_prompt: "Resume el texto",
        }),
      );
    });
  });

  it("keeps interface settings when saving from the Prompts tab", async () => {
    const user = userEvent.setup();
    mockUpdateSettings.mockResolvedValue(undefined);
    renderDialog(<ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>);

    await user.click(screen.getByText("Prompts"));

    const system = screen.getByLabelText("System Prompt");
    const form = system.closest("form") as HTMLFormElement;
    await user.click(within(form).getByRole("button", { name: /guardar/i }));

    await waitFor(() => {
      expect(mockUpdateSettings).toHaveBeenCalledWith(
        expect.objectContaining({
          font_size: "16",
          max_window_tokens: "10000",
          message_page_size: "50",
          system_prompt: "Eres Valet",
          archivist_prompt: "Eres un archivista",
          collapse_prompt: "Resume el texto",
        }),
      );
    });
  });

  it("editing only the Archivist prompt keeps the other prompts unchanged", async () => {
    const user = userEvent.setup();
    mockUpdateSettings.mockResolvedValue(undefined);
    renderDialog(<ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>);

    await user.click(screen.getByText("Prompts"));
    await user.click(screen.getByText("Archivist"));

    const archivist = screen.getByLabelText("Archivist Prompt");
    await user.clear(archivist);
    await user.type(archivist, "Nuevo archivist");

    const form = archivist.closest("form") as HTMLFormElement;
    await user.click(within(form).getByRole("button", { name: /guardar/i }));

    await waitFor(() => {
      expect(mockUpdateSettings).toHaveBeenCalledWith(
        expect.objectContaining({
          archivist_prompt: "Nuevo archivist",
          system_prompt: "Eres Valet",
          collapse_prompt: "Resume el texto",
        }),
      );
    });
  });

  it("renders API Keys tab with three password fields", async () => {
    const user = userEvent.setup();
    renderDialog(<ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>);

    await user.click(screen.getByText("API Keys"));

    expect(screen.getByText("OpenWeatherMap API Key")).toBeInTheDocument();
    expect(screen.getByText("Google Places API Key")).toBeInTheDocument();
    expect(screen.getByText("Brave Search API Key")).toBeInTheDocument();
  });

  it("calls resetToDefaults when clicking restore button", async () => {
    const user = userEvent.setup();
    mockResetToDefaults.mockResolvedValue(undefined);
    renderDialog(<ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>);

    await user.click(screen.getByText("Interfaz"));

    await user.click(screen.getByRole("button", { name: /restaurar/i }));

    await waitFor(() => {
      expect(mockResetToDefaults).toHaveBeenCalled();
    });

    expect(
      await screen.findByText("Valores por defecto restaurados"),
    ).toBeInTheDocument();
  });

  it("shows error message when settings save fails", async () => {
    const user = userEvent.setup();
    mockUpdateSettings.mockRejectedValue(new Error("boom"));
    renderDialog(<ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>);

    await user.click(screen.getByText("Interfaz"));

    await user.click(screen.getByRole("button", { name: /guardar/i }));

    await waitFor(() => {
      expect(
        screen.getByText("Error al guardar ajustes"),
      ).toBeInTheDocument();
    });
  });

  it("shows error message when resetToDefaults fails", async () => {
    const user = userEvent.setup();
    mockResetToDefaults.mockRejectedValue(new Error("boom"));
    renderDialog(<ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>);

    await user.click(screen.getByText("Interfaz"));

    await user.click(screen.getByRole("button", { name: /restaurar/i }));

    await waitFor(() => {
      expect(
        screen.getByText("Error al restaurar valores"),
      ).toBeInTheDocument();
    });
  });

  it("calls onClose when modal is cancelled", async () => {
    const onClose = vi.fn();
    renderDialog(<ProfileProvider><SettingsDialog visible={true} onClose={onClose} /></ProfileProvider>);

    // For antd Modal, the close button has aria-label "Close"
    const closeButton = screen.getByLabelText("Close");
    await userEvent.setup().click(closeButton);

    await waitFor(() => {
      expect(onClose).toHaveBeenCalled();
    });
  });

  // ════════════════════════════════════════════════════════════════
  // RED phase tests — validación de esquema de "Avatar URL"
  // ════════════════════════════════════════════════════════════════

  it("muestra error de validación y NO guarda con un esquema no permitido", async () => {
    const user = userEvent.setup();
    mockUpdateProfile.mockResolvedValue(undefined);
    renderDialog(
      <ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>,
    );

    const avatarInput = screen.getByLabelText("Avatar URL");
    await user.clear(avatarInput);
    await user.type(avatarInput, "javascript:alert(1)");

    await user.click(screen.getByRole("button", { name: /guardar/i }));

    await waitFor(() => {
      expect(
        document.querySelector(".ant-form-item-explain-error"),
      ).not.toBeNull();
    });
    expect(mockUpdateProfile).not.toHaveBeenCalled();
  });

  it("guarda una URL https válida sin error de validación", async () => {
    const user = userEvent.setup();
    mockUpdateProfile.mockResolvedValue(undefined);
    renderDialog(
      <ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>,
    );

    const avatarInput = screen.getByLabelText("Avatar URL");
    await user.clear(avatarInput);
    await user.type(avatarInput, "https://example.com/me.png");

    await user.click(screen.getByRole("button", { name: /guardar/i }));

    await waitFor(() => {
      expect(mockUpdateProfile).toHaveBeenCalledWith(
        expect.objectContaining({ avatar_url: "https://example.com/me.png" }),
      );
    });
    expect(document.querySelector(".ant-form-item-explain-error")).toBeNull();
  });

  it("acepta Avatar URL vacío y guarda sin error de validación", async () => {
    const user = userEvent.setup();
    mockUpdateProfile.mockResolvedValue(undefined);
    renderDialog(
      <ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>,
    );

    const avatarInput = screen.getByLabelText("Avatar URL");
    await user.clear(avatarInput);

    await user.click(screen.getByRole("button", { name: /guardar/i }));

    await waitFor(() => {
      expect(mockUpdateProfile).toHaveBeenCalled();
    });
    expect(document.querySelector(".ant-form-item-explain-error")).toBeNull();
  });

  it("acepta una ruta relativa en Avatar URL y la guarda", async () => {
    const user = userEvent.setup();
    mockUpdateProfile.mockResolvedValue(undefined);
    renderDialog(
      <ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>,
    );

    const avatarInput = screen.getByLabelText("Avatar URL");
    await user.clear(avatarInput);
    await user.type(avatarInput, "/avatars/me.png");

    await user.click(screen.getByRole("button", { name: /guardar/i }));

    await waitFor(() => {
      expect(mockUpdateProfile).toHaveBeenCalledWith(
        expect.objectContaining({ avatar_url: "/avatars/me.png" }),
      );
    });
    expect(document.querySelector(".ant-form-item-explain-error")).toBeNull();
  });

  it("persiste el Avatar URL recortado de espacios", async () => {
    const user = userEvent.setup();
    mockUpdateProfile.mockResolvedValue(undefined);
    renderDialog(
      <ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>,
    );

    const avatarInput = screen.getByLabelText("Avatar URL");
    await user.clear(avatarInput);
    await user.type(avatarInput, "  https://example.com/me.png  ");

    await user.click(screen.getByRole("button", { name: /guardar/i }));

    await waitFor(() => {
      expect(mockUpdateProfile).toHaveBeenCalledWith(
        expect.objectContaining({ avatar_url: "https://example.com/me.png" }),
      );
    });
  });

  it("rechaza una URL relativa al protocolo", async () => {
    const user = userEvent.setup();
    mockUpdateProfile.mockResolvedValue(undefined);
    renderDialog(
      <ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>,
    );

    const avatarInput = screen.getByLabelText("Avatar URL");
    await user.clear(avatarInput);
    await user.type(avatarInput, "//evil.com/a.png");

    await user.click(screen.getByRole("button", { name: /guardar/i }));

    await waitFor(() => {
      expect(
        document.querySelector(".ant-form-item-explain-error"),
      ).not.toBeNull();
    });
    expect(mockUpdateProfile).not.toHaveBeenCalled();
  });

  it("rechaza un valor con un tabulador embebido", async () => {
    const user = userEvent.setup();
    mockUpdateProfile.mockResolvedValue(undefined);
    renderDialog(
      <ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>,
    );

    const avatarInput = screen.getByLabelText("Avatar URL");
    fireEvent.change(avatarInput, { target: { value: "java\tscript:alert(1)" } });

    await user.click(screen.getByRole("button", { name: /guardar/i }));

    await waitFor(() => {
      expect(
        document.querySelector(".ant-form-item-explain-error"),
      ).not.toBeNull();
    });
    expect(mockUpdateProfile).not.toHaveBeenCalled();
  });

  it.each([
    "javascript:alert(1)",
    "data:image/png;base64,AAA",
    "file:///etc/passwd",
    "ftp://x/a.png",
    "//evil.com/a.png",
  ])("rechaza el esquema/valor no permitido %s", async (value) => {
    const user = userEvent.setup();
    mockUpdateProfile.mockResolvedValue(undefined);
    renderDialog(
      <ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>,
    );

    const avatarInput = screen.getByLabelText("Avatar URL");
    fireEvent.change(avatarInput, { target: { value } });

    await user.click(screen.getByRole("button", { name: /guardar/i }));

    await waitFor(() => {
      expect(
        document.querySelector(".ant-form-item-explain-error"),
      ).not.toBeNull();
    });
    expect(mockUpdateProfile).not.toHaveBeenCalled();
  });

  // ════════════════════════════════════════════════════════════════
  // RED phase tests — pestaña "Memoria" (cuatro mandos numéricos)
  // ════════════════════════════════════════════════════════════════

  it("renders the Memoria tab with the four memory knobs", async () => {
    const user = userEvent.setup();
    renderDialog(<ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>);

    await user.click(screen.getByText("Memoria"));

    expect(screen.getByLabelText("MEMORY_HALF_LIFE_DAYS")).toBeInTheDocument();
    expect(screen.getByLabelText("SIMILARITY_THRESHOLD")).toBeInTheDocument();
    expect(screen.getByLabelText("RAG_BUDGET_TOKENS")).toBeInTheDocument();
    expect(screen.getByLabelText("MEMORY_KNN_CANDIDATES")).toBeInTheDocument();
  });

  it("loads the memory knob values from the settings response", async () => {
    const user = userEvent.setup();
    mockSettings = {
      ...mockSettings,
      MEMORY_HALF_LIFE_DAYS: "90",
      RAG_BUDGET_TOKENS: "800",
    };
    renderDialog(<ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>);

    await user.click(screen.getByText("Memoria"));

    expect(screen.getByLabelText("MEMORY_HALF_LIFE_DAYS")).toHaveValue("90");
    expect(screen.getByLabelText("RAG_BUDGET_TOKENS")).toHaveValue("800");
  });

  it("saves the four memory knobs via updateSettings", async () => {
    const user = userEvent.setup();
    mockUpdateSettings.mockResolvedValue(undefined);
    renderDialog(<ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>);

    await user.click(screen.getByText("Memoria"));

    const halfLife = screen.getByLabelText("MEMORY_HALF_LIFE_DAYS");
    await user.clear(halfLife);
    await user.type(halfLife, "120");

    const threshold = screen.getByLabelText("SIMILARITY_THRESHOLD");
    await user.clear(threshold);
    await user.type(threshold, "0.7");

    const budget = screen.getByLabelText("RAG_BUDGET_TOKENS");
    await user.clear(budget);
    await user.type(budget, "1000");

    const knn = screen.getByLabelText("MEMORY_KNN_CANDIDATES");
    await user.clear(knn);
    await user.type(knn, "30");

    const form = halfLife.closest("form") as HTMLFormElement;
    await user.click(within(form).getByRole("button", { name: /guardar/i }));

    await waitFor(() => {
      expect(mockUpdateSettings).toHaveBeenCalledWith(
        expect.objectContaining({
          MEMORY_HALF_LIFE_DAYS: "120",
          SIMILARITY_THRESHOLD: "0.7",
          RAG_BUDGET_TOKENS: "1000",
          MEMORY_KNN_CANDIDATES: "30",
        }),
      );
    });

    expect(await screen.findByText("Ajustes guardados")).toBeInTheDocument();
  });

  it("renders the Memoria persistente tab", () => {
    renderDialog(<ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>);
    expect(screen.getByText("Memoria persistente")).toBeInTheDocument();
  });

  // ════════════════════════════════════════════════════════════════
  // RED phase tests — sub-pestaña "Consolidator" y aviso de placeholders
  // ════════════════════════════════════════════════════════════════

  it("renders the Consolidator sub-tab showing the current prompt", async () => {
    const user = userEvent.setup();
    renderDialog(<ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>);

    await user.click(screen.getByText("Prompts"));
    expect(screen.getByText("Consolidator")).toBeInTheDocument();

    await user.click(screen.getByText("Consolidator"));

    expect(screen.getByLabelText("Consolidator Prompt")).toHaveValue(
      "Consolida {{ ESTADO_ACTUAL }} con {{ BLOQUE_DE_MENSAJES }}",
    );
    expect(screen.queryByText(/Faltan placeholders/i)).not.toBeInTheDocument();
  });

  it("edits and saves consolidator_prompt keeping the rest of the settings", async () => {
    const user = userEvent.setup();
    mockUpdateSettings.mockResolvedValue(undefined);
    renderDialog(<ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>);

    await user.click(screen.getByText("Prompts"));
    await user.click(screen.getByText("Consolidator"));

    const area = screen.getByLabelText("Consolidator Prompt");
    await user.clear(area);
    await user.type(area, "Nuevo consolidator");

    const form = area.closest("form") as HTMLFormElement;
    await user.click(within(form).getByRole("button", { name: /guardar/i }));

    await waitFor(() => {
      expect(mockUpdateSettings).toHaveBeenCalledWith(
        expect.objectContaining({
          consolidator_prompt: "Nuevo consolidator",
          system_prompt: "Eres Valet",
          archivist_prompt: "Eres un archivista",
          collapse_prompt: "Resume el texto",
          font_size: "16",
          max_window_tokens: "10000",
        }),
      );
    });
  });

  it("warns about the missing placeholder naming it and still saves", async () => {
    const user = userEvent.setup();
    mockUpdateSettings.mockResolvedValue(undefined);
    renderDialog(<ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>);

    await user.click(screen.getByText("Prompts"));
    await user.click(screen.getByText("Consolidator"));

    const area = screen.getByLabelText("Consolidator Prompt");
    await user.clear(area);
    // user-event interpreta las llaves como descriptores de tecla; `fireEvent`
    // permite fijar el valor literal con placeholders.
    fireEvent.change(area, { target: { value: "Solo {{ ESTADO_ACTUAL }}" } });

    expect(
      await screen.findByText(/BLOQUE_DE_MENSAJES/),
    ).toBeInTheDocument();

    const form = area.closest("form") as HTMLFormElement;
    await user.click(within(form).getByRole("button", { name: /guardar/i }));

    await waitFor(() => {
      expect(mockUpdateSettings).toHaveBeenCalledWith(
        expect.objectContaining({
          consolidator_prompt: "Solo {{ ESTADO_ACTUAL }}",
        }),
      );
    });
  });

  it("does not warn when the prompt contains both placeholders", async () => {
    const user = userEvent.setup();
    renderDialog(<ProfileProvider><SettingsDialog visible={true} onClose={vi.fn()} /></ProfileProvider>);

    await user.click(screen.getByText("Prompts"));
    await user.click(screen.getByText("Consolidator"));

    expect(screen.getByLabelText("Consolidator Prompt")).toHaveValue(
      "Consolida {{ ESTADO_ACTUAL }} con {{ BLOQUE_DE_MENSAJES }}",
    );
    expect(screen.queryByText(/Faltan placeholders/i)).not.toBeInTheDocument();
  });
});