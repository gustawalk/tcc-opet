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

import { relaunch } from "@tauri-apps/plugin-process";

const mockedInvoke = vi.mocked(invoke);
const mockedRelaunch = vi.mocked(relaunch);
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

describe("Settings LAN shared storage", () => {
  afterEach(() => {
    cleanup();
    mockedRelaunch.mockReset();
  });

  beforeEach(() => {
    vi.clearAllMocks();
    mockedInvoke.mockImplementation((command) => {
      if (command === "get_settings") {
        return Promise.resolve({ companyName: "OPETS", cnpj: "", address: "", logoPath: "" });
      }
      if (command === "get_system_info") {
        return Promise.resolve({ databasePath: "/data/database.db", appVersion: "0.3.0", tauriVersion: "2", environment: "Teste" });
      }
      if (command === "get_automatic_backup_status") return Promise.resolve(automaticStatus);
      if (command === "get_storage_config") {
        return Promise.resolve({ databasePath: null, lanShared: false });
      }
      return Promise.resolve(null);
    });
  });

  const openLanSection = async () => {
    const user = userEvent.setup();
    renderSettings();
    await screen.findByText("Compartilhar na rede (LAN)");
    return user;
  };

  const openLanAccordion = async (user: ReturnType<typeof userEvent.setup>) => {
    const summary = (await screen.findByText("Compartilhar na rede (LAN)")).closest("summary");
    expect(summary).not.toBeNull();
    await user.click(summary as HTMLElement);
  };

  it("shows the experimental warning, saves the LAN toggle and restarts automatically", async () => {
    const user = await openLanSection();

    expect(screen.queryByText(/Modo experimental/)).not.toBeInTheDocument();
    const selectFolderButton = screen.getAllByRole("button", { name: "Selecionar pasta" })[0];
    expect(selectFolderButton).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Salvar" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Cancelar" })).not.toBeInTheDocument();

    await openLanAccordion(user);
    await user.click(screen.getByRole("checkbox", { name: "Ativar banco compartilhado na rede" }));
    expect(screen.getByText(/Modo experimental/)).toBeInTheDocument();
    expect(screen.getByText("Alterações pendentes")).toBeInTheDocument();
    const saveButton = screen.getByRole("button", { name: "Salvar" });
    expect(saveButton).toBeEnabled();
    expect(screen.getByRole("button", { name: "Cancelar" })).toBeEnabled();

    await user.click(saveButton);
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("update_storage_config", {
        databasePath: null,
        lanShared: true,
      });
    });
    await waitFor(() => {
      expect(mockedRelaunch).toHaveBeenCalledTimes(1);
    });
    expect(toast.success).toHaveBeenCalledWith(
      expect.stringContaining("Reiniciando o aplicativo"),
    );
  });

  it("locks the database path controls when LAN is enabled", async () => {
    mockedInvoke.mockImplementation((command) => {
      if (command === "get_settings") {
        return Promise.resolve({ companyName: "OPETS", cnpj: "", address: "", logoPath: "" });
      }
      if (command === "get_system_info") {
        return Promise.resolve({ databasePath: "/data/database.db", appVersion: "0.3.0", tauriVersion: "2", environment: "Teste" });
      }
      if (command === "get_automatic_backup_status") return Promise.resolve(automaticStatus);
      if (command === "get_storage_config") {
        return Promise.resolve({ databasePath: "/share/database.db", lanShared: true });
      }
      return Promise.resolve(null);
    });

    await openLanSection();

    expect(await screen.findByText("/share/database.db")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Cancelar" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Salvar" })).not.toBeInTheDocument();
    const folderButtons = screen.getAllByRole("button", { name: "Selecionar pasta" });
    expect(folderButtons).toHaveLength(1);
    expect(screen.getByText(/o banco atual é o da pasta fixada pela rede/)).toBeInTheDocument();
    expect(screen.getByText("Fixado pela rede")).toBeInTheDocument();

    const user = userEvent.setup();
    await openLanAccordion(user);
    await user.click(screen.getByRole("checkbox", { name: "Ativar banco compartilhado na rede" }));
    const saveButton = screen.getByRole("button", { name: "Salvar" });
    expect(saveButton).toBeEnabled();
    await user.click(saveButton);
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("update_storage_config", {
        databasePath: "/share/database.db",
        lanShared: false,
      });
    });
  });

  it("shows Save and Cancel after picking a folder and Cancelar reverts to the saved path", async () => {
    mockedInvoke
      .mockImplementation((command) => {
        if (command === "get_settings") {
          return Promise.resolve({ companyName: "OPETS", cnpj: "", address: "", logoPath: "" });
        }
        if (command === "get_system_info") {
          return Promise.resolve({ databasePath: "/data/database.db", appVersion: "0.3.0", tauriVersion: "2", environment: "Teste" });
        }
        if (command === "get_automatic_backup_status") return Promise.resolve(automaticStatus);
        if (command === "get_storage_config") {
          return Promise.resolve({ databasePath: "/current/database.db", lanShared: false });
        }
        if (command === "select_database_directory") return Promise.resolve("/picked");
        return Promise.resolve(null);
      });

    const user = await openLanSection();

    const pathDisplay = async () => screen.findByText("/current/database.db");
    expect(await pathDisplay()).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Salvar" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Cancelar" })).not.toBeInTheDocument();

    const selectFolderButton = screen.getAllByRole("button", { name: "Selecionar pasta" })[0];
    await user.click(selectFolderButton);
    await waitFor(() => {
      expect(screen.getByText("/picked/database.db")).toBeInTheDocument();
    });
    expect(screen.getByText("Alterações pendentes")).toBeInTheDocument();
    const saveButton = screen.getByRole("button", { name: "Salvar" });
    const cancelButton = screen.getByRole("button", { name: "Cancelar" });
    expect(saveButton).toBeEnabled();
    expect(cancelButton).toBeEnabled();

    await user.click(cancelButton);
    await waitFor(async () => {
      expect(await pathDisplay()).toBeInTheDocument();
    });
    expect(screen.queryByRole("button", { name: "Salvar" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Cancelar" })).not.toBeInTheDocument();

    await openLanAccordion(user);
    await user.click(screen.getByRole("checkbox", { name: "Ativar banco compartilhado na rede" }));
    const toggleSaveButton = screen.getByRole("button", { name: "Salvar" });
    expect(toggleSaveButton).toBeEnabled();
    await user.click(toggleSaveButton);
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("update_storage_config", {
        databasePath: "/current/database.db",
        lanShared: true,
      });
    });
  });
});
