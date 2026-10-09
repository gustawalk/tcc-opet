import { Download, X } from "lucide-react";
import { toast } from "sonner";

export function showUpdateAvailableToast(
  version: string,
  actionLabel: string,
  onAction: () => void,
) {
  toast.custom((id) => (
    <div
      role="status"
      className="flex w-[min(24rem,calc(100vw-2rem))] items-start gap-3 rounded-xl border border-border border-l-4 border-l-sky-500 bg-card p-4 text-card-foreground shadow-lg"
    >
      <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-sky-100 text-sky-700 dark:bg-sky-950 dark:text-sky-300">
        <Download aria-hidden="true" className="h-4 w-4" />
      </span>
      <div className="min-w-0 flex-1 space-y-1">
        <p className="text-sm font-semibold">Nova atualização disponível</p>
        <p className="text-sm text-muted-foreground">
          O OpetS {version} chegou! Confira as novidades e melhorias.
        </p>
        <button
          type="button"
          className="mt-2 inline-flex min-h-8 items-center rounded-md bg-sky-50 px-2.5 text-sm font-medium text-sky-700 transition-colors hover:bg-sky-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sky-500 focus-visible:ring-offset-2 dark:bg-sky-950 dark:text-sky-300 dark:hover:bg-sky-900"
          onClick={() => {
            toast.dismiss(id);
            onAction();
          }}
        >
          {actionLabel}
        </button>
      </div>
      <button
        type="button"
        aria-label="Fechar aviso de atualização"
        className="rounded-md p-1 text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        onClick={() => toast.dismiss(id)}
      >
        <X aria-hidden="true" className="h-4 w-4" />
      </button>
    </div>
  ), { duration: 6000 });
}
