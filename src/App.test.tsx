import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { relaunch } from "@tauri-apps/plugin-process";
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App, { LanStartupError, UpdatePatchNotes } from "./App";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/app", () => ({ getVersion: vi.fn() }));
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch: vi.fn() }));
vi.mock("@tauri-apps/plugin-updater", () => ({ check: vi.fn() }));
vi.mock("@tanstack/react-query-devtools", () => ({
  ReactQueryDevtools: () => null,
}));
vi.mock("./layouts/MainLayout", () => ({
  MainLayout: ({ children }: { children: ReactNode }) => <>{children}</>,
}));
vi.mock("./components/shared/ServiceOrderDrawerProvider", () => ({
  ServiceOrderDrawerProvider: ({ children }: { children: ReactNode }) => <>{children}</>,
  useServiceOrderDrawer: () => ({ openServiceOrder: () => undefined }),
}));
vi.mock("./components/shared/CustomerDrawerProvider", () => ({
  CustomerDrawerProvider: ({ children }: { children: ReactNode }) => <>{children}</>,
}));
vi.mock("./components/shared/AutomaticBackupProgress", () => ({
  AutomaticBackupProgress: () => null,
}));

const mockedInvoke = vi.mocked(invoke);

describe("UpdatePatchNotes", () => {
  afterEach(() => {
    cleanup();
    localStorage.removeItem("opets.pending-update-patch-notes");
  });

  it("renders formatted notes after a matching update", async () => {
    localStorage.setItem("opets.pending-update-patch-notes", JSON.stringify({
      version: "0.6.0",
      body: "## Novidades\n\n- **Status** mais rápido",
    }));
    vi.mocked(getVersion).mockResolvedValue("0.6.0");

    render(<UpdatePatchNotes />);

    const dialog = await screen.findByRole("dialog", { name: "Atualização concluída" });
    expect(await screen.findByRole("heading", { name: "Novidades" })).toBeInTheDocument();
    expect(dialog.querySelector("li strong")?.textContent).toBe("Status");
  });
});

describe("LanStartupError", () => {
  beforeEach(() => vi.clearAllMocks());
  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it("keeps the app open and returns safely to Local mode", async () => {
    mockedInvoke.mockImplementation((command) => {
      if (command === "get_lan_mode_config") {
        return Promise.resolve({
          config: {
            mode: "client",
            hostPort: 8743,
            clientUrl: "https://192.168.1.10:8743",
            clientDeviceName: "Balcão 2",
            clientToken: "token",
            clientCertificateFingerprint: "blake3:fingerprint",
            clientCertificatePem: "certificate",
          },
          activeMode: "client",
          restartRequired: false,
          storageReady: false,
        });
      }
      return Promise.resolve(null);
    });
    const retry = vi.fn();
    const user = userEvent.setup();

    render(
      <LanStartupError
        error={{ en: "Host unreachable", pt: "O computador host está indisponível." }}
        onRetry={retry}
      />,
    );

    expect(screen.getByRole("heading", { name: "Não foi possível conectar ao computador host" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Tentar novamente" }));
    expect(retry).toHaveBeenCalledOnce();

    await user.click(screen.getByRole("button", { name: "Usar dados locais" }));
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("update_lan_mode_config", {
        config: expect.objectContaining({
          mode: "local",
          clientUrl: null,
          clientToken: null,
          clientCertificateFingerprint: null,
          clientCertificatePem: null,
        }),
      });
      expect(relaunch).toHaveBeenCalledOnce();
    });
  });

  it("blocks a running Client session when the host connection is lost", async () => {
    vi.useFakeTimers();
    let connectionChecks = 0;
    mockedInvoke.mockImplementation((command) => {
      if (command === "get_lan_mode_config") {
        return Promise.resolve({
          config: {
            mode: "client",
            hostPort: 8743,
            clientUrl: "https://192.168.1.10:8743",
            clientDeviceName: "Balcão 2",
            clientToken: "token",
            clientCertificateFingerprint: "blake3:fingerprint",
            clientCertificatePem: "certificate",
          },
          activeMode: "client",
          restartRequired: false,
          storageReady: false,
        });
      }
      if (command === "check_lan_client_connection") {
        connectionChecks += 1;
        return connectionChecks === 1
          ? Promise.resolve({ ok: true })
          : Promise.reject({ pt: "O computador host está indisponível." });
      }
      return Promise.resolve(null);
    });

    render(<App />);
    await act(async () => undefined);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(5_000);
    });

    expect(screen.getByRole("heading", { name: "Não foi possível conectar ao computador host" })).toBeInTheDocument();
  });

  it("shows LAN connection progress while startup checks the host", async () => {
    mockedInvoke.mockImplementation((command) => {
      if (command === "get_lan_mode_config") {
        return Promise.resolve({
          config: {
            mode: "client",
            hostPort: 8743,
            clientUrl: "https://192.168.1.10:8743",
            clientDeviceName: "Balcão 2",
            clientToken: "token",
            clientCertificateFingerprint: "blake3:fingerprint",
            clientCertificatePem: "certificate",
          },
          activeMode: "client",
          restartRequired: false,
          storageReady: false,
        });
      }
      if (command === "check_lan_client_connection") return new Promise(() => undefined);
      return Promise.resolve(null);
    });

    render(<App />);

    expect(await screen.findByText(/Verificando conexão com o computador host/)).toBeInTheDocument();
  });

  it("retries the Host connection up to three times", async () => {
    let attempts = 0;
    mockedInvoke.mockImplementation((command) => {
      if (command === "get_lan_mode_config") {
        return Promise.resolve({
          config: { mode: "client", hostPort: 8743 },
          activeMode: "client",
          restartRequired: false,
          storageReady: false,
        });
      }
      if (command === "check_lan_client_connection") {
        attempts += 1;
        return attempts < 3
          ? Promise.reject({ pt: "O computador host está indisponível." })
          : Promise.resolve({ ok: true });
      }
      return Promise.resolve(null);
    });

    render(<App />);

    await waitFor(() => expect(attempts).toBe(3), { timeout: 4_000 });
    expect(screen.queryByRole("heading", { name: "Não foi possível conectar ao computador host" })).not.toBeInTheDocument();
  });
});
