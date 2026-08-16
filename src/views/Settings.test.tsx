import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { toast } from "sonner";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Settings } from "@/views/Settings";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn() }));
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch: vi.fn() }));
vi.mock("@tauri-apps/plugin-updater", () => ({ check: vi.fn() }));
vi.mock("sonner", () => ({ toast: { success: vi.fn(), error: vi.fn() } }));

const mockedInvoke = vi.mocked(invoke);
const automaticStatus = {
  enabled: false,
  destination: "/backups",
  intervalHours: 24,
  nextBackupAt: null,
  lastAttemptAt: null,
  lastSuccessAt: "2026-08-14T06:16:43Z",
  lastVerifiedAt: "2026-08-14T06:16:43Z",
  lastError: null,
  lastBackupPath: "/backups/opets-auto-20260814-031643.osbkp",
  lastBackupSizeBytes: 245760,
  running: false,
  progressPercent: 0,
  phase: null,
};

function renderSettings() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter>
        <Settings />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("Settings automatic backup", () => {
  afterEach(cleanup);

  beforeEach(() => {
    vi.clearAllMocks();
    mockedInvoke.mockImplementation((command, args) => {
      if (command === "get_settings") {
        return Promise.resolve({ companyName: "OPETS", cnpj: "", address: "", logoPath: "" });
      }
      if (command === "get_system_info") {
        return Promise.resolve({ databasePath: "/data/database.db", appVersion: "0.3.0", tauriVersion: "2", environment: "Teste" });
      }
      if (command === "get_automatic_backup_status") return Promise.resolve(automaticStatus);
      if (command === "update_automatic_backup_settings") {
        const settings = (args as { settings: typeof automaticStatus }).settings;
        return Promise.resolve({ ...automaticStatus, ...settings });
      }
      return Promise.resolve(null);
    });
  });

  it("places automatic backup after manual actions and activates from the checkbox", async () => {
    const user = userEvent.setup();
    renderSettings();

    const importButton = await screen.findByRole("button", { name: "Importar Backup" });
    const summary = screen.getByText("Backup automático").closest("summary");
    expect(summary).not.toBeNull();
    expect(importButton.compareDocumentPosition(summary as Node) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();

    await user.click(summary as HTMLElement);
    expect(screen.getByRole("button", { name: "Salvar configurações" })).toBeDisabled();
    expect(screen.getByText("Retenção leve: 7 pontos diários e 4 semanais.")).toBeInTheDocument();

    await user.click(screen.getByRole("checkbox", { name: "Ativar backup automático" }));
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("update_automatic_backup_settings", {
        settings: { enabled: true, destination: "/backups", intervalHours: 24 },
      });
    });
  });

  it("warns only when the interval exceeds 48 hours", async () => {
    const user = userEvent.setup();
    renderSettings();
    await user.click((await screen.findByText("Backup automático")).closest("summary") as HTMLElement);
    await user.click(screen.getByRole("checkbox", { name: "Ativar backup automático" }));
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("update_automatic_backup_settings", {
        settings: { enabled: true, destination: "/backups", intervalHours: 24 },
      });
    });
    const interval = screen.getByLabelText("Intervalo em horas");

    await user.clear(interval);
    await user.type(interval, "48");
    expect(screen.queryByText(/Intervalos acima de 48 horas/)).not.toBeInTheDocument();

    await user.clear(interval);
    await user.type(interval, "49");
    expect(screen.getByText(/Intervalos acima de 48 horas/)).toBeInTheDocument();
  });

  it("does not show the saving label nor success toast when activated from the checkbox", async () => {
    const user = userEvent.setup();
    let resolveUpdate: ((value: typeof automaticStatus) => void) | undefined;
    mockedInvoke.mockImplementation((command) => {
      if (command === "get_settings") {
        return Promise.resolve({ companyName: "OPETS", cnpj: "", address: "", logoPath: "" });
      }
      if (command === "get_system_info") {
        return Promise.resolve({ databasePath: "/data/database.db", appVersion: "0.3.0", tauriVersion: "2", environment: "Teste" });
      }
      if (command === "get_automatic_backup_status") return Promise.resolve(automaticStatus);
      if (command === "update_automatic_backup_settings") {
        return new Promise((resolve) => {
          resolveUpdate = resolve;
        });
      }
      return Promise.resolve(null);
    });

    renderSettings();
    await user.click((await screen.findByText("Backup automático")).closest("summary") as HTMLElement);
    await user.click(screen.getByRole("checkbox", { name: "Ativar backup automático" }));

    expect(screen.getByRole("button", { name: "Salvar configurações" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Salvando..." })).not.toBeInTheDocument();

    resolveUpdate?.({ ...automaticStatus, enabled: true });
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("update_automatic_backup_settings", {
        settings: { enabled: true, destination: "/backups", intervalHours: 24 },
      });
    });
    expect(toast.success).not.toHaveBeenCalled();
    expect(screen.queryByRole("button", { name: "Salvando..." })).not.toBeInTheDocument();
  });
});

describe("Settings appearance", () => {
  const originalMatchMedia = window.matchMedia;

  beforeEach(() => {
    vi.clearAllMocks();
    window.localStorage.clear();
    document.documentElement.classList.remove("dark");
    document.documentElement.style.removeProperty("--font-scale");
    window.matchMedia = vi.fn().mockReturnValue({
      matches: false,
      media: "(prefers-color-scheme: dark)",
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    } as unknown as MediaQueryList);
    mockedInvoke.mockImplementation((command) => {
      if (command === "get_settings") {
        return Promise.resolve({ companyName: "OPETS", cnpj: "", address: "", logoPath: "" });
      }
      if (command === "get_system_info") {
        return Promise.resolve({ databasePath: "/data/database.db", appVersion: "0.3.0", tauriVersion: "2", environment: "Teste" });
      }
      if (command === "get_automatic_backup_status") return Promise.resolve(automaticStatus);
      return Promise.resolve(null);
    });
  });

  afterEach(() => {
    cleanup();
    window.matchMedia = originalMatchMedia;
  });

  it("defaults the theme select to system and the font scale to standard", async () => {
    renderSettings();
    const themeSelect = await screen.findByRole("combobox", { name: "Tema" });
    expect(themeSelect).toHaveTextContent("Sistema");
    expect(screen.getByRole("combobox", { name: "Tamanho da fonte" })).toHaveTextContent("Padrão");
  });

  it("changes the theme to dark from the select", async () => {
    const user = userEvent.setup();
    renderSettings();
    await user.click(await screen.findByRole("combobox", { name: "Tema" }));
    await user.click(await screen.findByRole("option", { name: "Escuro" }));

    expect(document.documentElement.classList.contains("dark")).toBe(true);
    expect(window.localStorage.getItem("opets-theme")).toBe("dark");
  });

  it("changes the font scale to large from the select", async () => {
    const user = userEvent.setup();
    renderSettings();
    await user.click(await screen.findByRole("combobox", { name: "Tamanho da fonte" }));
    await user.click(await screen.findByRole("option", { name: "Grande" }));

    expect(document.documentElement.style.getPropertyValue("--font-scale")).toBe("1.1");
    expect(window.localStorage.getItem("opets-font-scale")).toBe("lg");
  });
});
