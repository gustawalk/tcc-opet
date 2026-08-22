import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { relaunch } from "@tauri-apps/plugin-process";
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

describe("Settings LAN host and client", () => {
  afterEach(cleanup);

  const installModeMock = (activeMode: "local" | "host" | "client") => {
    mockedInvoke.mockImplementation((command) => {
      if (command === "get_settings") {
        return Promise.resolve({ companyName: "OPETS", cnpj: "", address: "", logoPath: "" });
      }
      if (command === "get_system_info") {
        return Promise.resolve({ databasePath: "/data/database.db", appVersion: "0.3.2", tauriVersion: "2", environment: "Teste" });
      }
      if (command === "get_lan_mode_config") {
        return Promise.resolve({
          config: {
            mode: activeMode,
            hostPort: 8743,
            clientUrl: activeMode === "client" ? "https://192.168.1.10:8743" : null,
            clientDeviceName: activeMode === "client" ? "Balcão 2" : null,
            clientCertificateFingerprint: activeMode === "client" ? "blake3:abc123" : null,
          },
          activeMode,
          restartRequired: false,
          storageReady: activeMode !== "client",
        });
      }
      if (command === "get_lan_host_status") {
        return Promise.resolve({
          running: true,
          address: "192.168.1.10:8743",
          verificationCode: "123456|blake3:abc123",
          certificateFingerprint: "blake3:abc123",
          startupError: null,
        });
      }
      if (command === "list_lan_devices") {
        return Promise.resolve([
          {
            id: "device-1",
            name: "Balcão 2",
            appVersion: "0.3.2",
            createdAt: "2026-08-20T10:00:00Z",
            lastSeenAt: "2026-08-21T10:00:00Z",
            revokedAt: null,
          },
        ]);
      }
      if (command === "check_lan_client_connection") {
        return Promise.reject({ en: "Unreachable", pt: "Host indisponível" });
      }
      if (command === "get_automatic_backup_status") return Promise.resolve(automaticStatus);
      return Promise.resolve(null);
    });
  };

  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
  });

  it("shows host address, encrypted verification code, port, and running status", async () => {
    installModeMock("host");
    renderSettings();

    expect(await screen.findByText("Servidor LAN")).toBeInTheDocument();
    expect(screen.getByText("Ativo")).toBeInTheDocument();
    expect(screen.getByText("https://192.168.1.10:8743", { exact: false })).toBeInTheDocument();
    expect(screen.getByText("123456")).toBeInTheDocument();
    expect(screen.getByText("blake3:abc123")).toBeInTheDocument();
    expect(screen.getByLabelText("Porta local")).toHaveValue(8743);
    expect(screen.getByText("Balcão 2")).toBeInTheDocument();
  });

  it("confirms and revokes a paired device", async () => {
    const user = userEvent.setup();
    installModeMock("host");
    renderSettings();

    await user.click(await screen.findByRole("button", { name: "Revogar Balcão 2" }));
    expect(screen.getByText(/perderá acesso na próxima solicitação/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Confirmar revogação" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("revoke_lan_device", { id: "device-1" });
    });
  });

  it("shows pinned encryption, disconnected blocking, and host-only storage rules", async () => {
    installModeMock("client");
    renderSettings();

    expect(await screen.findByLabelText("Impressão digital do host")).toHaveValue("blake3:abc123");
    expect(await screen.findByText(/Leituras e alterações permanecem bloqueadas/)).toBeInTheDocument();
    expect(screen.getByText(/disponíveis somente no computador host/)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Importar Backup" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Resetar Todos os Dados" })).not.toBeInTheDocument();
    expect(screen.getByRole("checkbox", { name: "Baixar backup remoto automaticamente" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Alterar pasta do banco" })).not.toBeInTheDocument();
  });

  it("confirms a local database folder before saving and restarting", async () => {
    const user = userEvent.setup();
    installModeMock("local");
    vi.mocked(open).mockResolvedValue("/dados/opets");
    renderSettings();

    await user.click(await screen.findByRole("button", { name: "Alterar pasta do banco" }));
    expect(screen.getByText("Alterar pasta do banco de dados?")).toBeInTheDocument();
    expect(screen.getByText("/dados/opets/database.db", { exact: false })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Confirmar e reiniciar" }));
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("update_database_directory", {
        directory: "/dados/opets",
      });
      expect(relaunch).toHaveBeenCalled();
    });
  });

  it("downloads a host-created encrypted backup to the client-selected path", async () => {
    const user = userEvent.setup();
    installModeMock("client");
    vi.mocked(save).mockResolvedValue("/tmp/remote.osbkp");
    renderSettings();

    await user.click(await screen.findByRole("button", { name: "Exportar Backup" }));
    await user.click(screen.getByRole("button", { name: "Continuar" }));
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("download_lan_remote_backup", {
        destination: "/tmp/remote.osbkp",
        passphrase: "",
      });
    });
  });

  it("pairs a client and restarts after persisting the pinned credentials", async () => {
    const user = userEvent.setup();
    installModeMock("client");
    renderSettings();
    await user.type(await screen.findByLabelText("Código de pareamento"), "123456");
    expect(screen.getByLabelText("Impressão digital do host")).toHaveValue("blake3:abc123");
    await user.click(screen.getByRole("button", { name: "Parear e reiniciar" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("pair_lan_client", {
        url: "https://192.168.1.10:8743",
        deviceName: "Balcão 2",
        verificationCode: "123456|blake3:abc123",
      });
      expect(relaunch).toHaveBeenCalled();
    });
  });

  it.each([
    ["URL inválida", { en: "Invalid URL", pt: "O endereço deve usar HTTPS." }],
    ["código inválido", { en: "Invalid code", pt: "O código de pareamento é inválido." }],
    ["certificado alterado", { en: "Certificate changed", pt: "O certificado do host mudou." }],
    ["versão diferente", { en: "Version mismatch", pt: "As versões devem ser iguais." }],
  ])("reports %s without switching storage", async (_label, error) => {
    const user = userEvent.setup();
    installModeMock("client");
    const original = mockedInvoke.getMockImplementation();
    mockedInvoke.mockImplementation((command, args) => {
      if (command === "pair_lan_client") return Promise.reject(error);
      return original ? original(command, args) : Promise.resolve(null);
    });
    renderSettings();
    await user.type(await screen.findByLabelText("Código de pareamento"), "bad");
    await user.clear(screen.getByLabelText("Impressão digital do host"));
    await user.type(screen.getByLabelText("Impressão digital do host"), "blake3:bad");
    await user.click(screen.getByRole("button", { name: "Parear e reiniciar" }));

    await waitFor(() => expect(toast.error).toHaveBeenCalled());
    expect(relaunch).not.toHaveBeenCalled();
  });
});
