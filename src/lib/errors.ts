import { toast } from "sonner";

interface AppError {
  en: string;
  pt: string;
}

function isAppError(err: unknown): err is AppError {
  return (
    typeof err === "object" &&
    err !== null &&
    "pt" in err &&
    "en" in err &&
    typeof (err as AppError).pt === "string"
  );
}

export function getErrorMessage(err: unknown, fallback?: string): string {
  if (isAppError(err)) return err.pt;
  if (err instanceof Error) return err.message;
  if (typeof err === "string") return err;
  return fallback ?? "Ocorreu um erro inesperado.";
}

export function toastError(err: unknown, fallback?: string) {
  toast.error(getErrorMessage(err, fallback));
}

export function toastSuccess(msg: string) {
  toast.success(msg);
}

export async function copyToClipboard(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}
