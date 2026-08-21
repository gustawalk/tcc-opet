import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { configureDataClient } from "./data-client";
import {
  loadLanRemoteBackupSettings,
  runDueLanRemoteBackup,
  saveLanRemoteBackupSettings,
} from "./lan-backup";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

describe("LAN remote backup scheduler", () => {
  beforeEach(() => {
    localStorage.clear();
    configureDataClient("client");
    vi.mocked(invoke).mockReset().mockResolvedValue({ attachmentCount: 0 });
  });

  it("downloads a due host-created backup and records the successful run", async () => {
    saveLanRemoteBackupSettings({
      enabled: true,
      destination: "/backups",
      intervalHours: 24,
    });
    const now = new Date("2026-08-21T12:00:00Z");

    await expect(runDueLanRemoteBackup(now)).resolves.toBe(true);
    expect(invoke).toHaveBeenCalledWith("run_scheduled_lan_remote_backup", {
      destinationDirectory: "/backups",
    });
    expect(loadLanRemoteBackupSettings().lastRunAt).toBe(now.toISOString());
  });

  it("does not download before the configured interval", async () => {
    saveLanRemoteBackupSettings({
      enabled: true,
      destination: "/backups",
      intervalHours: 24,
      lastRunAt: "2026-08-21T11:00:00Z",
    });

    await expect(runDueLanRemoteBackup(new Date("2026-08-21T12:00:00Z"))).resolves.toBe(false);
    expect(invoke).not.toHaveBeenCalled();
  });
});
