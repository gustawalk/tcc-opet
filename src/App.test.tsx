import { invoke } from "@tauri-apps/api/core";
import { relaunch } from "@tauri-apps/plugin-process";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { LanStartupError } from "./App";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch: vi.fn() }));
vi.mock("@tauri-apps/plugin-updater", () => ({ check: vi.fn() }));

const mockedInvoke = vi.mocked(invoke);

describe("LanStartupError", () => {
  beforeEach(() => vi.clearAllMocks());

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
});
