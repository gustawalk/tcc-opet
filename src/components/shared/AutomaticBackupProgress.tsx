import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { DatabaseBackup, LoaderCircle } from "lucide-react";
import type { AutomaticBackupProgress as ProgressEvent, AutomaticBackupStatus } from "@/lib/types";

const INITIAL_PROGRESS: ProgressEvent = {
  running: false,
  percent: 0,
  phase: "idle",
  message: "",
};

export function AutomaticBackupProgress() {
  const [progress, setProgress] = useState<ProgressEvent>(INITIAL_PROGRESS);
  const eventVersion = useRef(0);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    let statusInterval: ReturnType<typeof setInterval> | undefined;

    const syncStatus = (versionBeforeStatus = eventVersion.current) => {
      void invoke<AutomaticBackupStatus>("get_automatic_backup_status")
        .then((status) => {
          if (!disposed && eventVersion.current === versionBeforeStatus) {
            setProgress(status.running ? {
              running: true,
              percent: status.progressPercent,
              phase: status.phase ?? "preparing",
              message: "Backup automático em andamento, aguarde alguns instantes.",
            } : INITIAL_PROGRESS);
          }
        })
        .catch(() => undefined);
    };

    void listen<ProgressEvent>("automatic-backup-progress", (event) => {
      if (!disposed) {
        eventVersion.current += 1;
        setProgress(event.payload);
      }
    }).then((dispose) => {
      if (disposed) {
        dispose();
        return;
      }
      unlisten = dispose;
      syncStatus();
    }).catch(() => {
      if (disposed) return;
      syncStatus();
      statusInterval = setInterval(() => syncStatus(), 1000);
    });

    return () => {
      disposed = true;
      unlisten?.();
      if (statusInterval) clearInterval(statusInterval);
    };
  }, []);

  if (!progress.running) return null;

  return (
    <div
      className="fixed inset-0 z-[120] flex items-center justify-center bg-background/85 p-6 backdrop-blur-sm"
      role="status"
      aria-live="assertive"
      aria-label="Backup automático em andamento"
    >
      <div className="flex w-full max-w-sm flex-col items-center gap-5 rounded-xl border bg-card p-8 text-center shadow-2xl">
        <div className="relative">
          <DatabaseBackup className="h-10 w-10 text-primary" />
          <LoaderCircle className="absolute -right-3 -top-3 h-5 w-5 animate-spin text-primary" />
        </div>
        <div className="space-y-1">
          <p className="font-semibold">Backup automático em andamento</p>
          <p className="text-sm text-muted-foreground">
            Backup automático em andamento, aguarde alguns instantes.
          </p>
        </div>
        <div className="w-full space-y-2">
          <div className="h-2 overflow-hidden rounded-full bg-muted">
            <div
              className="h-full bg-primary transition-[width] duration-300"
              style={{ width: `${Math.max(0, Math.min(100, progress.percent))}%` }}
            />
          </div>
          <p className="text-xs text-muted-foreground">
            {progress.percent}% concluído. {progress.message}
          </p>
        </div>
      </div>
    </div>
  );
}
