import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

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
    settings: {
      max_window_tokens: "10000",
      system_prompt: "Eres Valet",
      archivist_prompt: "Eres un archivista",
      collapse_prompt: "Resume el texto",
      font_size: "16",
      message_page_size: "50",
      openweather_api_key: "",
      google_places_api_key: "",
      brave_search_api_key: "",
    },
    loading: false,
    saving: false,
    error: null,
    updateSettings: mockUpdateSettings,
    resetToDefaults: mockResetToDefaults,
  })),
}));

import { SettingsDialog } from "../components/SettingsDialog";

describe("SettingsDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("is not visible when visible=false", () => {
    render(<SettingsDialog visible={false} onClose={vi.fn()} />);
    expect(screen.queryByText("⚙️ Settings")).not.toBeInTheDocument();
  });

  it("renders modal with correct title when visible", () => {
    render(<SettingsDialog visible={true} onClose={vi.fn()} />);
    expect(screen.getByText("⚙️ Settings")).toBeInTheDocument();
  });

  it("renders Perfil tab by default with name and avatar fields", () => {
    render(<SettingsDialog visible={true} onClose={vi.fn()} />);
    expect(screen.getByText("Perfil")).toBeInTheDocument();
    expect(screen.getByLabelText("Nombre")).toBeInTheDocument();
    expect(screen.getByLabelText("Avatar URL")).toBeInTheDocument();
  });

  it("saves profile when submitting Perfil tab", async () => {
    const user = userEvent.setup();
    mockUpdateProfile.mockResolvedValue(undefined);
    render(<SettingsDialog visible={true} onClose={vi.fn()} />);

    const nameInput = screen.getByLabelText("Nombre");
    await user.clear(nameInput);
    await user.type(nameInput, "Juan");

    await user.click(screen.getByRole("button", { name: /guardar/i }));

    await waitFor(() => {
      expect(mockUpdateProfile).toHaveBeenCalledWith(
        expect.objectContaining({ name: "Juan" }),
      );
    });
  });

  it("shows error message when profile save fails", async () => {
    const user = userEvent.setup();
    mockUpdateProfile.mockRejectedValue(new Error("fail"));
    render(<SettingsDialog visible={true} onClose={vi.fn()} />);

    await user.click(screen.getByRole("button", { name: /guardar/i }));

    await waitFor(() => {
      expect(screen.getByText("Error al actualizar perfil")).toBeInTheDocument();
    });
  });

  it("renders Interfaz tab with font size, context window, and page size", async () => {
    const user = userEvent.setup();
    render(<SettingsDialog visible={true} onClose={vi.fn()} />);

    // Click on Interfaz tab
    await user.click(screen.getByText("Interfaz"));

    expect(screen.getByText("Tamaño de fuente")).toBeInTheDocument();
    expect(screen.getByText("Ventana de contexto (tokens)")).toBeInTheDocument();
    expect(screen.getByText("Tamaño de página")).toBeInTheDocument();
  });

  it("renders Prompts tab with System, Archivist and Collapse sub-tabs", async () => {
    const user = userEvent.setup();
    render(<SettingsDialog visible={true} onClose={vi.fn()} />);

    await user.click(screen.getByText("Prompts"));

    expect(screen.getByText("System")).toBeInTheDocument();
    expect(screen.getByText("Archivist")).toBeInTheDocument();
    expect(screen.getByText("Collapse")).toBeInTheDocument();
  });

  it("shows system_prompt when opening the System sub-tab", async () => {
    const user = userEvent.setup();
    render(<SettingsDialog visible={true} onClose={vi.fn()} />);

    await user.click(screen.getByText("Prompts"));
    await user.click(screen.getByText("System"));

    expect(screen.getByLabelText("System Prompt")).toHaveValue("Eres Valet");
  });

  it("shows archivist_prompt when opening the Archivist sub-tab", async () => {
    const user = userEvent.setup();
    render(<SettingsDialog visible={true} onClose={vi.fn()} />);

    await user.click(screen.getByText("Prompts"));
    await user.click(screen.getByText("Archivist"));

    expect(screen.getByLabelText("Archivist Prompt")).toHaveValue(
      "Eres un archivista",
    );
  });

  it("shows collapse_prompt when opening the Collapse sub-tab", async () => {
    const user = userEvent.setup();
    render(<SettingsDialog visible={true} onClose={vi.fn()} />);

    await user.click(screen.getByText("Prompts"));
    await user.click(screen.getByText("Collapse"));

    expect(screen.getByLabelText("Collapse Prompt")).toHaveValue(
      "Resume el texto",
    );
  });

  it("saves the three prompts together via updateSettings", async () => {
    const user = userEvent.setup();
    mockUpdateSettings.mockResolvedValue(undefined);
    render(<SettingsDialog visible={true} onClose={vi.fn()} />);

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
  });

  it("keeps the loaded prompts when saving from the Interfaz tab", async () => {
    const user = userEvent.setup();
    mockUpdateSettings.mockResolvedValue(undefined);
    render(<SettingsDialog visible={true} onClose={vi.fn()} />);

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
    render(<SettingsDialog visible={true} onClose={vi.fn()} />);

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
    render(<SettingsDialog visible={true} onClose={vi.fn()} />);

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
    render(<SettingsDialog visible={true} onClose={vi.fn()} />);

    await user.click(screen.getByText("API Keys"));

    expect(screen.getByText("OpenWeatherMap API Key")).toBeInTheDocument();
    expect(screen.getByText("Google Places API Key")).toBeInTheDocument();
    expect(screen.getByText("Brave Search API Key")).toBeInTheDocument();
  });

  it("calls resetToDefaults when clicking restore button", async () => {
    const user = userEvent.setup();
    mockResetToDefaults.mockResolvedValue(undefined);
    render(<SettingsDialog visible={true} onClose={vi.fn()} />);

    await user.click(screen.getByText("Interfaz"));

    await user.click(screen.getByRole("button", { name: /restaurar/i }));

    await waitFor(() => {
      expect(mockResetToDefaults).toHaveBeenCalled();
    });
  });

  it("calls onClose when modal is cancelled", async () => {
    const onClose = vi.fn();
    render(<SettingsDialog visible={true} onClose={onClose} />);

    // For antd Modal, the close button has aria-label "Close"
    const closeButton = screen.getByLabelText("Close");
    await userEvent.setup().click(closeButton);

    await waitFor(() => {
      expect(onClose).toHaveBeenCalled();
    });
  });
});