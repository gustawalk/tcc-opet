import { invoke } from "@tauri-apps/api/core";
import { getDataClientMode } from "./data-client";

const STORAGE_KEY = "opets.lan-remote-backup";

export interface LanRemoteBackupSettings {
  enabled: boolean;
  destination: string;
  intervalHours: number;
  lastRunAt?: string;
}

export function loadLanRemoteBackupSettings(): LanRemoteBackupSettings {
  const fallback = { enabled: false, destination: "", intervalHours: 24 };
  try {
    const parsed = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "null");
    return parsed && typeof parsed === "object" ? { ...fallback, ...parsed } : fallback;
  } catch {
    return fallback;
  }
}

export function saveLanRemoteBackupSettings(settings: LanRemoteBackupSettings): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
}

export async function runDueLanRemoteBackup(now = new Date()): Promise<boolean> {
  if (getDataClientMode() !== "client") return false;
  const settings = loadLanRemoteBackupSettings();
  if (!settings.enabled || !settings.destination) return false;
  const last = settings.lastRunAt ? new Date(settings.lastRunAt).getTime() : 0;
  if (now.getTime() - last < settings.intervalHours * 60 * 60 * 1000) return false;
  await invoke("run_scheduled_lan_remote_backup", {
    destinationDirectory: settings.destination,
  });
  saveLanRemoteBackupSettings({ ...settings, lastRunAt: now.toISOString() });
  return true;
}
