import { useEffect, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { DatabaseBackup, LoaderCircle } from "lucide-react";
import type { AutomaticBackupProgress as ProgressEvent, AutomaticBackupStatus } from "@/lib/types";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { toastError } from "@/lib/errors";

const INITIAL_PROGRESS: ProgressEvent = {
  running: false,
  percent: 0,
  phase: "idle",
  message: "",
};

export function AutomaticBackupProgress() {
  const queryClient = useQueryClient();
  const [progress, setProgress] = useState<ProgressEvent>(INITIAL_PROGRESS);
  const eventVersion = useRef(0);
  const backendWasRunning = useRef(false);
  const lastObservedStatus = useRef<string | null>(null);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    let statusInterval: ReturnType<typeof setInterval> | undefined;
    let statusRequestPending = false;

    const syncStatus = (versionBeforeStatus = eventVersion.current) => {
      if (statusRequestPending) return;
      statusRequestPending = true;
      void invoke<AutomaticBackupStatus>("get_automatic_backup_status")
        .then((status) => {
          if (!disposed && eventVersion.current === versionBeforeStatus) {
            const statusVersion = `${status.lastAttemptAt ?? ""}|${status.lastSuccessAt ?? ""}|${status.lastError ?? ""}`;
            if (lastObservedStatus.current !== null && lastObservedStatus.current !== statusVersion) {
              void queryClient.invalidateQueries({ queryKey: ["automatic-backup-status"] });
              if (status.lastError) toastError(status.lastError);
            }
            lastObservedStatus.current = statusVersion;
            if (!status.running && backendWasRunning.current) {
              void queryClient.invalidateQueries({ queryKey: ["automatic-backup-status"] });
            }
            backendWasRunning.current = status.running;
            setProgress(status.running ? {
              running: true,
              percent: status.progressPercent,
              phase: status.phase ?? "preparing",
              message: "Backup automático em andamento, aguarde alguns instantes.",
            } : INITIAL_PROGRESS);
          }
        })
        .catch(() => undefined)
        .finally(() => { statusRequestPending = false; });
    };

    void listen<ProgressEvent>("automatic-backup-progress", (event) => {
      if (!disposed) {
        eventVersion.current += 1;
        backendWasRunning.current = event.payload.running;
        setProgress(event.payload);
        if (!event.payload.running) {
          void queryClient.invalidateQueries({ queryKey: ["automatic-backup-status"] });
          if (event.payload.phase === "failed") {
            toastError(event.payload.message, "O backup automático falhou.");
          }
        }
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
  }, [queryClient]);

  if (!progress.running) return null;

  return (
    <Dialog open>
      <DialogContent
        showClose={false}
        className="flex max-w-sm flex-col items-center gap-5 p-8 text-center"
        onEscapeKeyDown={(event) => event.preventDefault()}
        onPointerDownOutside={(event) => event.preventDefault()}
      >
        <div className="relative">
          <DatabaseBackup className="h-10 w-10 text-primary" />
          <LoaderCircle className="absolute -right-3 -top-3 h-5 w-5 animate-spin text-primary" />
        </div>
        <div className="space-y-1">
          <DialogTitle>Backup automático em andamento</DialogTitle>
          <DialogDescription>
            Backup automático em andamento, aguarde alguns instantes.
          </DialogDescription>
        </div>
        <div className="w-full space-y-2">
          <div
            className="h-2 overflow-hidden rounded-full bg-muted"
            role="progressbar"
            aria-label="Progresso do backup automático"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={Math.max(0, Math.min(100, progress.percent))}
          >
            <div
              className="h-full bg-primary transition-[width] duration-300"
              style={{ width: `${Math.max(0, Math.min(100, progress.percent))}%` }}
            />
          </div>
          <p className="text-xs text-muted-foreground" aria-live="polite">
            {progress.percent}% concluído. {progress.message}
          </p>
        </div>
      </DialogContent>
    </Dialog>
  );
}
